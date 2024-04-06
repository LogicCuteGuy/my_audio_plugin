use serde::{Deserialize, Deserializer, Serialize, Serializer};


#[derive(Serialize, Deserialize)]
pub struct NoteTables {
    pub i2t: [i8; 84],

    //can use in midi only
    pub im2t: [i8; 84],
}
//Make New Desige
impl Default for NoteTables {
    fn default() -> Self {
        Self {
            i2t: [0; 84],
            im2t: [0; 84]
        }
    }
}
