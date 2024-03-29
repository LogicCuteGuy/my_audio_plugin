use nih_plug::params::{BoolParam, FloatParam, IntParam, Params};
use nih_plug::prelude::FloatRange;

pub struct HZCalculator {
    pub hz_center: f32,

    pub hz_tuning: f32
}

#[derive(Params)]
pub struct HZCalculatorParams {
    #[id = "hz_center"]
    pub hz_center: FloatParam,

    #[id = "hz_tuning"]
    pub hz_tuning: FloatParam,
}

impl Default for HZCalculatorParams {
    fn default() -> Self {
        Self {
            hz_center: FloatParam::new("Hz Center", 440.0, FloatRange::Linear{ min: 400.0, max: 500.0 }),
            hz_tuning: FloatParam::new("Hz Tuning", 440.0, FloatRange::Linear{ min: 400.0, max: 500.0 })
        }
    }
}

impl HZCalculator {

}

impl Default for HZCalculator {
    fn default() -> Self {
        Self { hz_center: 440.0, hz_tuning: 440.0 }
    }
}