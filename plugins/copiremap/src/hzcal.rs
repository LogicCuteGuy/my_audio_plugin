
pub struct HZCalculator {
    pub hz_input: f32,

    pub hz_output: f32,
}

impl HZCalculator {

}

impl Default for HZCalculator {
    fn default() -> Self {
        Self { hz_input: 440.0, hz_output: 440.0}
    }
}