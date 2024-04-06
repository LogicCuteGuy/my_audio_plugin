use nih_plug::formatters;
use nih_plug::params::{FloatParam,  Params};
use nih_plug::prelude::FloatRange;

#[derive(Params)]
pub struct HZCalculatorParams {
    #[id = "hz_center"]
    pub hz_center: FloatParam,

    #[id = "hz_tuning"]
    pub hz_tuning: FloatParam,
}

impl Default for HZCalculatorParams {
    fn default() -> Self {
        Self {
            hz_center: FloatParam::new("Hz Center", 440.0, FloatRange::Linear{ min: 415.3046976, max: 466.1637615 })
                .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
                .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
            hz_tuning: FloatParam::new("Hz Tuning", 440.0, FloatRange::Linear{ min: 415.3046976, max: 466.1637615 })
                .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
                .with_string_to_value(formatters::s2v_f32_hz_then_khz())
        }
    }
}

pub fn hz_cal_clh(note: u8, center_hz: &mut f32, low_pass: &mut f32, high_pass: &mut f32, hz_cal: f32) {
    *center_hz = hz_cal * 2.0_f32.powf((note as f32 - 45.0) / 12.0);
    *low_pass = ((hz_cal * 2.0_f32.powf((note as f32 - 44.0) / 12.0)) + *center_hz) / 2.0;
    *high_pass = ((hz_cal * 2.0_f32.powf((note as f32 - 46.0) / 12.0))+ *center_hz) / 2.0;
}

pub fn hz_cal_tlh(note: u8, note_pitch: i8, pitch_tune_hz: &mut f32, low_pass: &mut f32, high_pass: &mut f32, hz_center: f32, hz_tuning: f32) {
    *pitch_tune_hz = 2.0_f32.powf((note_pitch as f32 + (12.0 * (hz_center / hz_tuning).log2())) / 12.0) ;
    let tune_note_hz: f32 = hz_tuning * 2.0_f32.powf(((note - 45) as i8 + note_pitch) as f32 / 12.0);
    *low_pass = ((hz_tuning * 2.0_f32.powf(((note - 44) as i8 + note_pitch) as f32 / 12.0)) + &tune_note_hz) / 2.0;
    *high_pass = ((hz_tuning * 2.0_f32.powf(((note - 46) as i8 + note_pitch) as f32 / 12.0)) + &tune_note_hz) / 2.0;
}