pub fn hz_cal_clh(note: u8, center_hz: &mut f32, low_pass: &mut f32, high_pass: &mut f32, hz_cal: f32) {
    *center_hz = hz_cal * 2.0_f32.powf((note as f32 - 69.0) / 12.0);
    *low_pass = ((hz_cal * 2.0_f32.powf((note as f32 - 70.0) / 12.0)) + *center_hz) / 2.0;
    *high_pass = ((hz_cal * 2.0_f32.powf((note as f32 - 68.0) / 12.0))+ *center_hz) / 2.0;
}

pub fn hz_cal_tlh(note: u8, note_pitch: i8, pitch_tune_hz: &mut f32, low_pass: &mut f32, high_pass: &mut f32, hz_center: f32, hz_tuning: f32) {
    let note_pitch = if note_pitch == -128 {0} else {note_pitch};
    *pitch_tune_hz = 2.0_f32.powf((note_pitch as f32 + (12.0 * (hz_center / hz_tuning).log2())) / 12.0);
    let tune_note_hz: f32 = hz_tuning * 2.0_f32.powf(((note as i8) - 69 + note_pitch) as f32 / 12.0);
    *low_pass = ((hz_tuning * 2.0_f32.powf(((note as i8) - 70 + note_pitch) as f32 / 12.0)) + &tune_note_hz) / 2.0;
    *high_pass = ((hz_tuning * 2.0_f32.powf(((note as i8) - 68 + note_pitch) as f32 / 12.0)) + &tune_note_hz) / 2.0;
}