
pub struct NoteTable {
    i2t: [u8; 128],

    //can use in midi only
    im2t: [u8; 128],
}

impl Default for NoteTable {
    fn default() -> Self {
        let mut i2t = [0; 128];
        let mut im2t = [0; 128];
        for i in 0..128 {
            i2t[i] = i as u8;
            im2t[i] = i as u8;
        }
        Self {
            i2t,
            im2t
        }
    }
}