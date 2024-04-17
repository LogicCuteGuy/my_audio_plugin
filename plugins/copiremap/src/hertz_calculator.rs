pub fn hz_cal_clh(note: u8, center_hz: &mut f32, hz_cal: f32) {
    *center_hz = hz_cal * 2.0_f32.powf((note as f32 - 69.0) / 12.0);
}

pub fn hz_cal_tlh(note: u8, note_pitch: i8, pitch_tune_hz: &mut f32, bandpass: &mut f32, hz_center: f32, hz_tuning: f32) {
    let note_pitch = if note_pitch == -128 {0} else {note_pitch};
    *pitch_tune_hz = 2.0_f32.powf((note_pitch as f32 + (12.0 * (hz_center / hz_tuning).log2())) / 12.0);
    *bandpass = hz_tuning * 2.0_f32.powf(((note as i8) - 69 + note_pitch) as f32 / 12.0);
}