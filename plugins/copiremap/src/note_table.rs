use std::sync::Arc;
use nih_plug::params::persist::PersistentField;
use serde::{Deserialize, Serialize};
use crossbeam::atomic::AtomicCell;

#[derive(Serialize, Deserialize)]
pub struct NoteTables {
    #[serde(with = "nih_plug::params::persist::serialize_atomic_cell")]
    pub i2t: AtomicCell<NoteTablesArray>,

    //can use in midi only
    #[serde(with = "nih_plug::params::persist::serialize_atomic_cell")]
    pub im2t: AtomicCell<NoteTablesArray>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct NoteTablesArray {
    #[serde(with = "serialize")]
    pub i104: [i8; 104]
}

impl<'a> PersistentField<'a, NoteTables> for Arc<NoteTables> {
    fn set(&self, new_value: NoteTables) {
        self.i2t.store(new_value.i2t.load());
        self.im2t.store(new_value.im2t.load());
    }

    fn map<F, R>(&self, f: F) -> R
        where
            F: Fn(&NoteTables) -> R,
    {
        f(self)
    }
}

impl Default for NoteTables {
    fn default() -> Self {
        Self {
            i2t: AtomicCell::new(NoteTablesArray { i104: [0; 104] }),
            im2t: AtomicCell::new(NoteTablesArray { i104: [0; 104] })
        }
    }
}

pub mod serialize {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S, T>(cell: &[T; 104], serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
            T: Serialize + Copy,
    {
        cell.serialize(serializer)
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<[T; 104], D::Error>
        where
            D: Deserializer<'de>,
            T: Deserialize<'de> + Copy,
    {
        let result: Result<T, _> = T::deserialize(deserializer);
        result.map(|val| [val; 104])
    }
}