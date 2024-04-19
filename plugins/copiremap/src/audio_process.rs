use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use nih_plug::audio_setup::BufferConfig;
use nih_plug::formatters;
use nih_plug::params::{BoolParam, FloatParam, IntParam, Params};
use nih_plug::prelude::{FloatRange, IntRange};
use nih_plug::util::db_to_gain;
use simple_eq::design::Curve;
use crate::{PluginParams};
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

    #[id = "in_key_gain"]
    pub in_key_gain: FloatParam,

    #[id = "off_key_gain"]
    pub off_key_gain: FloatParam,

    #[id = "pitch_shift_window_duration_ms"]
    pub pitch_shift_window_duration_ms: IntParam
}

impl AudioProcessParams {
    pub fn new(update_pitch_shift_over_sampling: Arc<AtomicBool>, update_pitch_shift_window_duration_ms: Arc<AtomicBool>, update_pitch_shift_and_after_bandpass: Arc<AtomicBool>) -> Self {
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
                true,
            ).with_callback({
                let update_pitch_shift_and_after_bandpass = update_pitch_shift_and_after_bandpass.clone();
                Arc::new(move |_| {
                    update_pitch_shift_and_after_bandpass.store(true, Ordering::Release);
                })
            }),
            pitch_shift_over_sampling: IntParam::new(
                "Pitch Shift Over Sampling",
                1,
                IntRange::Linear {
                    min: 1,
                    max: 8,
                }
            ).with_callback(
                {
                    Arc::new(move |_| {
                        update_pitch_shift_over_sampling.store(true, Ordering::Release);
                    })
                }
            ),
            in_key_gain: FloatParam::new(
                "In Key Gain",
                db_to_gain(0.0),
                FloatRange::Skewed {
                    min: db_to_gain(-48.0),
                    max: db_to_gain(12.0),
                    factor: FloatRange::gain_skew_factor(-48.0, 12.0),
                }
            ).with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            off_key_gain: FloatParam::new(
                "Off Key Gain",
                db_to_gain(0.0),
                FloatRange::Skewed {
                    min: db_to_gain(-48.0),
                    max: db_to_gain(12.0),
                    factor: FloatRange::gain_skew_factor(-48.0, 12.0),
                }
            ).with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            pitch_shift_window_duration_ms: IntParam::new(
                "Pitch Shift Window Duration",
                2,
                IntRange::Linear {
                    min: 1,
                    max: 20,
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

pub struct AudioProcess108 {
    bpf: MyFilter,
    tuning: MyPitch,
    delay: Delay,
    note: u8,
    center_hz: f32,
    tuning_hz: f32,
    note_pitch: i8
}

impl AudioProcess108 {
    pub fn reset(&mut self) {
        self.tuning.reset();
        self.delay.reset();
    }

    pub fn get_latency(&self) -> u32 {
        self.tuning.get_latency()
    }

    pub fn set_delay(&mut self, delay: u32) {
        self.delay.set_delay(delay);
    }

    pub fn setup(&mut self, params: Arc<PluginParams>, note: u8, buffer_config: &BufferConfig) {
        let note_pitch: i8 = params.note_table.i2t.load().i108[note as usize];
        hz_cal_clh(note, note_pitch, &mut self.center_hz, params.global.hz_center.value(), !params.audio_process.pitch_shift.value());
        self.bpf.set(Curve::Bandpass, self.center_hz, params.global.resonance.value(), 0.0, buffer_config.sample_rate);
        println!("{}", self.center_hz);
        let mut pitch_tune_hz: f32 = 0.0;
        let mut bandpass: f32 = 0.0;
        self.note_pitch = note_pitch;
        hz_cal_tlh(note, note_pitch, &mut pitch_tune_hz, &mut bandpass, params.global.hz_center.value(), params.global.hz_tuning.value(), !params.audio_process.pitch_shift.value());
        self.tuning_hz = pitch_tune_hz.clone();
        self.tuning.set_window_duration_ms(params.audio_process.pitch_shift_window_duration_ms.value() as u8, buffer_config.sample_rate, params.audio_process.pitch_shift_over_sampling.value() as u8, pitch_tune_hz);
        self.note = note;
    }

    pub fn set_pitch_shift_and_after_bandpass(&mut self, params: Arc<PluginParams>, note_pitch: i8, buffer_config: &BufferConfig) {
        let mut bandpass: f32 = 0.0;
        let mut pitch_tune_hz: f32 = 0.0;
        hz_cal_tlh(self.note, note_pitch, &mut pitch_tune_hz, &mut bandpass, params.global.hz_center.value(), params.global.hz_tuning.value(), !params.audio_process.pitch_shift.value());
        self.bpf.set(Curve::Bandpass, bandpass, params.global.resonance.value(), 0.0, buffer_config.sample_rate);
        self.note_pitch = note_pitch;
        self.tuning_hz = pitch_tune_hz.clone();
        self.tuning.set_pitch(pitch_tune_hz);
    }

    pub fn set_pitch_shift_over_sampling(&mut self, params: Arc<PluginParams>) {
        self.tuning.set_over_sampling(params.audio_process.pitch_shift_over_sampling.value() as u8);
    }

    pub fn set_pitch_shift_window_duration_ms(&mut self, params: Arc<PluginParams>, buffer_config: &BufferConfig) {
        let pitch: f32 = self.tuning.get_pitch();
        self.tuning.set_window_duration_ms(params.audio_process.pitch_shift_window_duration_ms.value() as u8, buffer_config.sample_rate, params.audio_process.pitch_shift_over_sampling.value() as u8, pitch);
    }

    pub fn set_bpf_center_hz(&mut self, params: Arc<PluginParams>, buffer_config: &BufferConfig) {
        let note_pitch: i8 = params.note_table.i2t.load().i108[self.note as usize];
        hz_cal_clh(self.note, note_pitch, &mut self.center_hz, params.global.hz_center.value(), !params.audio_process.pitch_shift.value());
        self.bpf.set(Curve::Bandpass, self.center_hz, params.global.resonance.value(), 0.0, buffer_config.sample_rate);
    }

    pub fn process(&mut self, input: f32, params: Arc<PluginParams>, audio_id: usize) -> f32 {
        let input: f32 = if self.note_pitch == 0 { input * params.audio_process.in_key_gain.value() } else { input * params.audio_process.off_key_gain.value()};
        let threshold: bool = input >= params.audio_process.threshold.value();
        let pitch: f32 = match params.audio_process.pitch_shift.value() && !(self.note_pitch == 0 || self.note_pitch == -128) {
            true => self.tuning.process(input, audio_id),
            false => self.delay.process(input, audio_id)
        };
        let bpf: f32 = self.bpf.process(pitch, audio_id);
        // output = if self.note_pitch == -128 { 0.0 } else { output };
        bpf
    }

    pub fn fn_update_pitch_shift_and_after_bandpass(params: Arc<PluginParams>, audio_process: &mut Vec<AudioProcess108>, buffer_config: &BufferConfig, note_pitch: [i8; 108]) {
        for (i, ap) in audio_process.iter_mut().enumerate() {
            ap.set_pitch_shift_and_after_bandpass(params.clone(), note_pitch[i], buffer_config);
        }
        println!("Doing {:?}", note_pitch)
    }
}

impl Default for AudioProcess108 {
    fn default() -> Self {
        Self {
            bpf: MyFilter::default(),
            tuning: MyPitch::default(),
            delay: Delay::default(),
            note: 0,
            center_hz: 0.0,
            tuning_hz: 0.0,
            note_pitch: 0,
        }
    }

}