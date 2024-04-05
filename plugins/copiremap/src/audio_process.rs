use nih_plug::formatters;
use nih_plug::params::{BoolParam, FloatParam, IntParam, Params};
use nih_plug::prelude::{FloatRange, IntRange, SmoothingStyle};
use nih_plug::util::db_to_gain;

pub struct AudioProcessNot {
    pub pitch_shift_window_duration_ms: u8,
}

#[derive(Params)]
pub struct AudioProcessParams {
    #[id = "threshold"]
    pub threshold: FloatParam,

    #[id = "order"]
    pub order: IntParam,

    #[id = "low_note_off"]
    pub low_note_off: IntParam,

    #[id = "high_note_off"]
    pub high_note_off: IntParam,

    #[id = "low_note_off_mute"]
    pub low_note_off_mute: BoolParam,

    #[id = "high_note_off_mute"]
    pub high_note_off_mute: BoolParam,

    #[id = "pitch_shift"]
    pub pitch_shift: BoolParam,

    #[id = "pitch_shift_over_sampling"]
    pub pitch_shift_over_sampling: IntParam,

    #[id = "after_pitch_shift_bandpass"]
    pub after_pitch_shift_bandpass: BoolParam,

    #[id = "bandwidth_after_pitch_shift_bandpass"]
    pub bandwidth_after_pitch_shift_bandpass: FloatParam,

    #[id = "order_after_pitch_shift_bandpass"]
    pub order_after_pitch_shift_bandpass: IntParam,

    #[id = "in_key_gain"]
    pub in_key_gain: FloatParam,

    #[id = "off_key_gain"]
    pub off_key_gain: FloatParam,
}

impl Default for AudioProcessParams {
    fn default() -> Self {
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
            order: IntParam::new(
                "Order",
                5,
                IntRange::Linear {
                    min: 0,
                    max: 15,
                },
            ),
            low_note_off: IntParam::new(
                "Low Note Off",
                0,
                IntRange::Linear {
                    min: 0,
                    max: 127,
                },
            ).with_value_to_string(formatters::v2s_i32_note_formatter())
                .with_string_to_value(formatters::s2v_i32_note_formatter()),
            high_note_off: IntParam::new(
                "High Note Off",
                127,
                IntRange::Linear {
                    min: 0,
                    max: 127,
                }
            ).with_value_to_string(formatters::v2s_i32_note_formatter())
                .with_string_to_value(formatters::s2v_i32_note_formatter()),
            low_note_off_mute: BoolParam::new(
                "Low Note Off Mute",
                false,
            ),
            high_note_off_mute: BoolParam::new(
                "High Note Off Mute",
                false,
            ),
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
            ),
            after_pitch_shift_bandpass: BoolParam::new(
                "After Pitch Shift Bandpass",
                false,
            ),
            bandwidth_after_pitch_shift_bandpass: FloatParam::new(
                "Bandwidth After Pitch Shift Bandpass",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                }
            ).with_unit("%")
                .with_smoother(SmoothingStyle::Linear(15.0))
                .with_value_to_string(formatters::v2s_f32_percentage(2))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            order_after_pitch_shift_bandpass: IntParam::new(
                "Order After Pitch Shift Bandpass",
                5,
                IntRange::Linear {
                    min: 0,
                    max: 15,
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
        }
    }
}