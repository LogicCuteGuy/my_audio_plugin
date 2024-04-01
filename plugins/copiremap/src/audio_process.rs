use nih_plug::params::{BoolParam, FloatParam, IntParam, Params};

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

    //is can't edit realtime
    #[id = "pitch_shift_window_duration_ms"]
    pub pitch_shift_window_duration_ms: IntParam,

    #[id = "pitch_shift_over_sampling"]
    pub pitch_shift_over_sampling: IntParam,

    #[id = "after_pitch_shift_bandpass"]
    pub after_pitch_shift_bandpass: BoolParam,

    #[id = "high_bandwidth_after_pitch_shift_bandpass"]
    pub high_bandwidth_after_pitch_shift_bandpass: FloatParam,

    #[id = "low_bandwidth_after_pitch_shift_bandpass"]
    pub low_bandwidth_after_pitch_shift_bandpass: FloatParam,

    #[id = "order_after_pitch_shift_bandpass"]
    pub order_after_pitch_shift_bandpass: FloatParam,

    #[id = "in_key_gain"]
    pub in_key_gain: FloatParam,

    #[id = "off_key_gain"]
    pub off_key_gain: FloatParam,

    #[id = "off_key"]
    pub off_key: BoolParam,
}