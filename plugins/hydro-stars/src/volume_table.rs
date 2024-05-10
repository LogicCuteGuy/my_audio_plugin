use std::sync::Arc;
use crossbeam::atomic::AtomicCell;
use nih_plug::params::persist::PersistentField;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct VolumeTable {
    #[serde(with = "atomic_cell_serializer")]
    pub vt: AtomicCell<Vec<f32>>,
}

impl<'a> PersistentField<'a, VolumeTable> for Arc<VolumeTable> {
    fn set(&self, new_value: VolumeTable) {
        self.vt.store(new_value.vt.take());
    }

    fn map<F, R>(&self, f: F) -> R
        where
            F: Fn(&VolumeTable) -> R,
    {
        f(self)
    }
}

impl Default for VolumeTable {
    fn default() -> Self {
        Self {
            vt: AtomicCell::new(vec![0.0; 128]),
        }
    }
}

mod atomic_cell_serializer {
    use serde::{Serializer, Deserializer, Serialize, Deserialize};
    use crossbeam::atomic::AtomicCell;

    pub fn serialize<S>(data: &AtomicCell<Vec<f32>>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        data.take().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<AtomicCell<Vec<f32>>, D::Error>
        where
            D: Deserializer<'de>,
    {
        Ok(AtomicCell::new(Vec::deserialize(deserializer)?))
    }
}