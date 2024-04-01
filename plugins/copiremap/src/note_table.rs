
pub struct NoteTable {
    i2t: [u8; 127],
}

impl Default for NoteTable {
    fn default() -> Self {
        let mut i2t = [0; 127];
        for i in 0..127 {
            i2t[i] = i as u8;
        }
        Self {
            i2t,
        }
    }
}