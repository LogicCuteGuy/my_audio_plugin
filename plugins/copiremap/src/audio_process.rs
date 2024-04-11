use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use nih_plug::formatters;
use nih_plug::params::{BoolParam, FloatParam, IntParam, Params};
use nih_plug::prelude::{FloatRange, IntRange};
use nih_plug::util::db_to_gain;
use iir_filters::filter_design::FilterType;
use crate::CoPiReMapPlugin;
use crate::delay::Delay;
use crate::filter::MyFilter;
use crate::hertz_calculator::{hz_cal_clh, hz_cal_tlh};
use crate::pitch::MyPitch;

#[derive(Params)]
pub struct AudioProcessParams {
    #[id = "threshold"]
    pub threshold: FloatParam,

    #[id = "pitch_shift"]
    pub pitch_shift: BoolParam,

    #[id = "pitch_shift_over_sampling"]
    pub pitch_shift_over_sampling: IntParam,

    #[id = "after_pitch_shift_bandpass"]
    pub after_pitch_shift_bandpass: BoolParam,

    #[id = "order_after_pitch_shift_bandpass"]
    pub order_after_pitch_shift_bandpass: IntParam,

    #[id = "in_key_gain"]
    pub in_key_gain: FloatParam,

    #[id = "off_key_gain"]
    pub off_key_gain: FloatParam,

    #[id = "pitch_shift_window_duration_ms"]
    pub pitch_shift_window_duration_ms: IntParam
}

impl AudioProcessParams {
    pub fn new(update_pitch_shift_and_after_bandpass: Arc<AtomicBool>, update_pitch_shift_over_sampling: Arc<AtomicBool>, update_pitch_shift_window_duration_ms: Arc<AtomicBool>) -> Self {
        Self {
            threshold: FloatParam::new(
                "Threshold",
                db_to_gain(0.0),
                FloatRange::Linear {
                    min: db_to_gain(-60.0),
                    max: db_to_gain(0.0),
                },
            ).with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            pitch_shift: BoolParam::new(
                "Pitch Shift",
                false,
            ),
            pitch_shift_over_sampling: IntParam::new(
                "Pitch Shift Over Sampling",
                1,
                IntRange::Linear {
                    min: 1,
                    max: 16,
                }
            ).with_callback(
                {
                    Arc::new(move |_| {
                        update_pitch_shift_over_sampling.store(true, Ordering::Release);
                    })
                }
            ),
            after_pitch_shift_bandpass: BoolParam::new(
                "After Pitch Shift Bandpass",
                false,
            ),
            order_after_pitch_shift_bandpass: IntParam::new(
                "Order After Pitch Shift Bandpass",
                5,
                IntRange::Linear {
                    min: 0,
                    max: 15,
                }
            ).with_callback(
                {
                    Arc::new(move |_| {
                        update_pitch_shift_and_after_bandpass.store(true, Ordering::Release);
                    })
                }
            ),
            in_key_gain: FloatParam::new(
                "In Key Gain",
                0.0,
                FloatRange::Skewed {
                    min: db_to_gain(-20.0),
                    max: db_to_gain(6.0),
                    factor: FloatRange::gain_skew_factor(-20.0, 6.0),
                }
            ).with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            off_key_gain: FloatParam::new(
                "Off Key Gain",
                db_to_gain(0.0),
                FloatRange::Skewed {
                    min: db_to_gain(-20.0),
                    max: db_to_gain(6.0),
                    factor: FloatRange::gain_skew_factor(-20.0, 6.0),
                }
            ).with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            pitch_shift_window_duration_ms: IntParam::new(
                "Order After Pitch Shift Bandpass",
                2,
                IntRange::Linear {
                    min: 1,
                    max: 200,
                }
            ).with_unit("ms")
                .with_callback(
                {
                    Arc::new(move |_| {
                        update_pitch_shift_window_duration_ms.store(true, Ordering::Release);
                    })
                }
            ),
        }
    }
}

pub struct AudioProcess {
    bpf: MyFilter,
    tuning: MyPitch,
    after_tune_bpf: MyFilter,
    delay: Delay,
    note: u8,
    center_hz: f32,
    tuning_hz: f32,
}

impl AudioProcess {
    pub fn reset(&mut self) {
        self.tuning.reset();
    }

    pub fn get_latency(&self) -> u32 {
        self.tuning.get_latency()
    }

    pub fn set_delay(&mut self, delay: u32) {
        self.delay.set_delay(delay);
    }

    pub fn setup(&mut self, plugin: &CoPiReMapPlugin, note: u8) {
        let mut lowpass: f32 = 0.0;
        let mut highpass: f32 = 0.0;
        hz_cal_clh(note, &mut self.center_hz, &mut lowpass, &mut highpass, plugin.params.global.hz_center.value());
        self.bpf.set_filter(plugin.params.global.order.value() as u8, FilterType::BandPass(lowpass, highpass), plugin.buffer_config.sample_rate);
        let mut pitch_tune_hz: f32 = 0.0;
        let mut lowpass: f32 = 0.0;
        let mut highpass: f32 = 0.0;
        let note_pitch: i8 = plugin.params.note_table.i2t.load().i84[(note - 24) as usize];
        hz_cal_tlh(note, note_pitch, &mut pitch_tune_hz, &mut lowpass, &mut highpass, plugin.params.global.hz_center.value(), plugin.params.global.hz_tuning.value());
        self.tuning_hz = pitch_tune_hz.clone();
        self.tuning.set_pitch(pitch_tune_hz);
        self.after_tune_bpf.set_filter(plugin.params.audio_process.order_after_pitch_shift_bandpass.value() as u8, FilterType::BandPass(lowpass, highpass), plugin.buffer_config.sample_rate);
        self.note = note;
    }

    pub fn set_pitch_shift_and_after_bandpass(&mut self, plugin: &CoPiReMapPlugin, note_pitch: i8) {
        let mut lowpass: f32 = 0.0;
        let mut highpass: f32 = 0.0;
        let mut pitch_tune_hz: f32 = 0.0;
        hz_cal_tlh(self.note, note_pitch, &mut pitch_tune_hz, &mut lowpass, &mut highpass, plugin.params.global.hz_center.value(), plugin.params.global.hz_tuning.value());
        self.tuning_hz = pitch_tune_hz.clone();
        self.tuning.set_pitch(pitch_tune_hz);
        self.after_tune_bpf.set_filter(plugin.params.audio_process.order_after_pitch_shift_bandpass.value() as u8, FilterType::BandPass(lowpass, highpass), plugin.buffer_config.sample_rate);
    }

    pub fn set_pitch_shift_over_sampling(&mut self, plugin: &CoPiReMapPlugin) {
        self.tuning.set_over_sampling(plugin.params.audio_process.pitch_shift_over_sampling.value() as u8);
    }

    pub fn set_pitch_shift_window_duration_ms(&mut self, plugin: &CoPiReMapPlugin) {
        let pitch: f32 = self.tuning.get_pitch();
        self.tuning.set_window_duration_ms(plugin.params.audio_process.pitch_shift_window_duration_ms.value() as u8, plugin.buffer_config.sample_rate, plugin.params.audio_process.pitch_shift_over_sampling.value() as u8, pitch);
    }

    pub fn set_bpf_center_hz(&mut self, plugin: &CoPiReMapPlugin) {
        let mut lowpass: f32 = 0.0;
        let mut highpass: f32 = 0.0;
        hz_cal_clh(self.note, &mut self.center_hz, &mut lowpass, &mut highpass, plugin.params.global.hz_center.value());
        self.bpf.set_filter(plugin.params.global.order.value() as u8, FilterType::BandPass(lowpass, highpass), plugin.buffer_config.sample_rate);
    }

    pub fn process(&mut self, input: [f32; 2], plugin: &CoPiReMapPlugin) -> [f32; 2] {
        let bpf: [f32; 2] = self.bpf.process(input);
        let threshold: bool = bpf[0] >= plugin.params.audio_process.threshold.value() || bpf[1] >= plugin.params.audio_process.threshold.value();
        let pitch: [f32; 2] = match plugin.params.audio_process.pitch_shift.value() && threshold {
            true => self.tuning.process(bpf),
            false => self.delay.process(bpf)
        };
        let after_tune_bpf: [f32; 2] = match plugin.params.audio_process.after_pitch_shift_bandpass.value() && threshold {
            true => self.after_tune_bpf.process(pitch),
            false => pitch
        };
        let note_pitch: i8 = plugin.params.note_table.i2t.load().i84[(self.note - 24) as usize];
        let mut output: [f32; 2] = [0.0, 0.0];
        output[0] = if note_pitch == 0 { after_tune_bpf[0] * plugin.params.audio_process.in_key_gain.value() } else { after_tune_bpf[0] * plugin.params.audio_process.off_key_gain.value()};
        output[1] = if note_pitch == 0 { after_tune_bpf[1] * plugin.params.audio_process.in_key_gain.value() } else { after_tune_bpf[1] * plugin.params.audio_process.off_key_gain.value()};
        output
    }

    pub fn custom_copy(&self) -> Self {
        Self {
            bpf: Default::default(),
            tuning: Default::default(),
            after_tune_bpf: Default::default(),
            delay: Default::default(),
            note: self.note,
            center_hz: self.center_hz,
            tuning_hz: self.tuning_hz,
        }
    }
}

impl Default for AudioProcess {
    fn default() -> Self {
        Self {
            bpf: MyFilter::default(),
            tuning: MyPitch::default(),
            after_tune_bpf: MyFilter::default(),
            delay: Delay::default(),
            note: 0,
            center_hz: 0.0,
            tuning_hz: 0.0,
        }
    }

}