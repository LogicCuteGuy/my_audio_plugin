use realfft::{RealFftPlanner};
use realfft::RealToComplex;
use realfft::ComplexToReal;
use realfft::num_complex::Complex;

use std::f32::consts::PI;
use std::f32::consts::TAU;
use std::sync::Arc; // = 2xPI

type SampleReal = f32;
const COMPLEX_ZERO: Complex<SampleReal> = Complex::new(0.0, 0.0);

/// See [`PitchShifter::new`] & [`PitchShifter::shift_pitch`]
pub struct PitchShifter {
    forward_fft: Arc<dyn RealToComplex<f32>>,
    inverse_fft: Arc<dyn ComplexToReal<f32>>,
    ffft_scratch_len: usize,
    ifft_scratch_len: usize,
    fft_scratch: Vec<Complex<SampleReal>>,
    fft_real: Vec<SampleReal>,
    fft_cplx: Vec<Complex<SampleReal>>,

    in_fifo: Vec<SampleReal>,
    out_fifo: Vec<SampleReal>,

    last_phase: Vec<SampleReal>,
    phase_sum: Vec<SampleReal>,
    windowing: Vec<SampleReal>,
    output_accumulator: Vec<SampleReal>,
    synthesized_frequency: Vec<SampleReal>,
    synthesized_magnitude: Vec<SampleReal>,

    frame_size: usize,
    overlap: usize,
    sample_rate: usize,

    //pitch

    fifo_latency: usize,
    half_frame_size: usize,
    shift: f32,
    expected: f32,
    pitch_weight: f32,
    oversamp_weight: f32,
    over_sampling: usize,
    step: usize,
    mean_expected: f32,
    bin_frequencies: SampleReal,
}

impl PitchShifter {
    /// Phase Vocoding works by extracting overlapping windows
    /// from a buffer and processing them individually before
    /// merging the results into the output buffer.
    ///
    /// You must set a duration in miliseconds for these windows;
    /// 50ms is a good value.
    ///
    /// The sample rate argument must correspond to the sample
    /// rate of the buffer(s) you will provide to
    /// [`PitchShifter::shift_pitch`], which is how many values
    /// correspond to one second of audio in the buffer.
    pub fn new(window_duration_ms: usize, sample_rate: usize, over_sampling: usize, shift: SampleReal) -> Self {
        let mut frame_size = sample_rate * window_duration_ms / 1000;
        frame_size += frame_size % 2;
        let fs_real = frame_size as SampleReal;

        let double_frame_size = frame_size * 2;

        let mut planner = RealFftPlanner::<f32>::new();
        let forward_fft = planner.plan_fft_forward(frame_size);
        let inverse_fft = planner.plan_fft_inverse(frame_size);
        let ffft_scratch_len = forward_fft.get_scratch_len();
        let ifft_scratch_len = inverse_fft.get_scratch_len();
        let scratch_len = ffft_scratch_len.max(ifft_scratch_len);

        let mut windowing = vec![0.0; frame_size];
        for k in 0..frame_size {
            windowing[k] = -0.5 * (TAU * (k as SampleReal) / fs_real).cos() + 0.5;
        }

        //pitch
        let shift = 2.0_f32.powf(shift / 12.0);
        let fs_real = frame_size as SampleReal;
        let half_frame_size = (frame_size / 2) + 1;

        let step = frame_size / over_sampling;
        let bin_frequencies = sample_rate as SampleReal / fs_real;
        let expected = TAU / (over_sampling as SampleReal);
        let fifo_latency = frame_size - step;
        println!("{}", fifo_latency);
        let mut overlap= 0;
        overlap = fifo_latency;

        let pitch_weight = shift * bin_frequencies;
        let oversamp_weight = ((over_sampling as SampleReal) / TAU) * pitch_weight;
        let mean_expected = expected / bin_frequencies;

        Self {
            forward_fft,
            inverse_fft,
            ffft_scratch_len,
            ifft_scratch_len,
            fft_scratch: vec![COMPLEX_ZERO; scratch_len],
            fft_real: vec![0.0; frame_size],
            fft_cplx: vec![COMPLEX_ZERO; half_frame_size],

            in_fifo: vec![0.0; frame_size],
            out_fifo: vec![0.0; frame_size],

            last_phase: vec![0.0; half_frame_size],
            phase_sum: vec![0.0; half_frame_size],
            windowing,
            output_accumulator: vec![0.0; double_frame_size],
            synthesized_frequency: vec![0.0; frame_size],
            synthesized_magnitude: vec![0.0; frame_size],

            frame_size,
            overlap,
            sample_rate,

            fifo_latency,
            half_frame_size,
            shift,
            expected,
            pitch_weight,
            oversamp_weight,
            over_sampling,
            step,
            mean_expected,
            bin_frequencies,
        }
    }

    pub fn pitch(&mut self, shift: SampleReal) {
        self.shift = 2.0_f32.powf(shift / 12.0);
        self.pitch_weight = self.shift * self.bin_frequencies;
        self.oversamp_weight = ((self.over_sampling as SampleReal) / TAU) * self.pitch_weight;
    }

    pub fn over_sampling(&mut self, over_sampling: usize) {
        self.step = self.frame_size / over_sampling;
        self.expected = TAU / (over_sampling as SampleReal);
        self.fifo_latency = self.frame_size - self.step;
        self.overlap = self.fifo_latency;
        self.oversamp_weight = ((over_sampling as SampleReal) / TAU) * self.pitch_weight;
        self.mean_expected = self.expected / self.bin_frequencies;
    }

    /// This is where the magic happens.
    ///
    /// The bigger `over_sampling`, the longer it will take to
    /// process, but the better the results. I put `16` in the
    /// `shift-wav` binary.
    ///
    /// `shift` is how many semitones to apply to the buffer.
    /// It is signed: a negative value will lower the tone and
    /// vice-versa.
    ///
    /// `in_b` is where the input buffer goes, and you must pass
    /// an output buffer of the same length in `out_b`.
    ///
    /// Note: It's actually not magic, sadly.
    pub fn process(&mut self, signal: SampleReal) -> SampleReal {
        self.in_fifo[self.overlap] = signal;
        let out = self.out_fifo[self.overlap - self.fifo_latency];
        self.overlap += 1;
        if self.overlap >= self.frame_size {
            self.overlap = self.fifo_latency;

            for k in 0..self.frame_size {
                self.fft_real[k] = self.in_fifo[k] * self.windowing[k];
            }

            let _ = self.forward_fft.process_with_scratch(
                &mut self.fft_real,
                &mut self.fft_cplx,
                &mut self.fft_scratch[..self.ffft_scratch_len],
            );//.unwrap();

            self.synthesized_magnitude.fill(0.0);
            self.synthesized_frequency.fill(0.0);

            for k in 0..self.half_frame_size {
                let k_real = k as SampleReal;
                let index = (k_real * self.shift).round() as usize;
                if index < self.half_frame_size {
                    let (magnitude, phase) = self.fft_cplx[k].to_polar();
                    let mut delta_phase = (phase - self.last_phase[k]) - k_real * self.expected;
                    // must not round here for some reason
                    let mut qpd = (delta_phase / PI) as i64;

                    if qpd >= 0 {
                        qpd += qpd & 1;
                    } else {
                        qpd -= qpd & 1;
                    }

                    delta_phase -= PI * qpd as SampleReal;
                    self.last_phase[k] = phase;
                    self.synthesized_magnitude[index] += magnitude;
                    self.synthesized_frequency[index] = k_real * self.pitch_weight + self.oversamp_weight * delta_phase;
                }
            }

            self.fft_cplx.fill(COMPLEX_ZERO);

            for k in 0..self.half_frame_size {
                self.phase_sum[k] += self.mean_expected * self.synthesized_frequency[k];

                let (sin, cos) = self.phase_sum[k].sin_cos();
                let magnitude = self.synthesized_magnitude[k];

                self.fft_cplx[k].im = sin * magnitude;
                self.fft_cplx[k].re = cos * magnitude;
            }

            let _ = self.inverse_fft.process_with_scratch(
                &mut self.fft_cplx,
                &mut self.fft_real,
                &mut self.fft_scratch[..self.ifft_scratch_len],
            );//.unwrap();

            let acc_oversamp: SampleReal = 2.0 / (self.half_frame_size * self.over_sampling) as SampleReal;

            for k in 0..self.frame_size {
                let product = self.windowing[k] * self.fft_real[k] * acc_oversamp;
                self.output_accumulator[k] += product / 2.0;
            }

            self.out_fifo[..self.step].copy_from_slice(&self.output_accumulator[..self.step]);
            self.output_accumulator.copy_within(self.step..(self.step + self.frame_size), 0);
            self.in_fifo.copy_within(self.step..(self.step + self.fifo_latency), 0);
        }
        out
    }
}