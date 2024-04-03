
pub struct NoteTable {
    pub i2t: [i8; 128],

    //can use in midi only
    pub im2t: [i8; 128],
}
//Make New Desige
impl Default for NoteTable {
    fn default() -> Self {
        Self {
            i2t: [0; 128],
            im2t: [0; 128]
        }
    }
}