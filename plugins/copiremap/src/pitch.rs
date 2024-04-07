use pitch_shift::PitchShifter;

pub struct MyPitch {
    pitch: [PitchShifter; 2]
}

impl MyPitch {

    pub fn set_window_duration_ms(&mut self, window_duration_ms: u8, sample_rate: f32, over_sampling: u8, shift: f32) {
        self.pitch[0] = PitchShifter::new(window_duration_ms, sample_rate as u32, over_sampling, shift);
        self.pitch[1] = PitchShifter::new(window_duration_ms, sample_rate as u32, over_sampling, shift);
    }

    pub fn set_pitch(&mut self, shift: f32) {
        self.pitch[0].pitch(shift);
        self.pitch[1].pitch(shift);
    }

    pub fn over_sampling(&mut self, over_sampling: u8) {
        self.pitch[0].over_sampling(over_sampling);
        self.pitch[1].over_sampling(over_sampling);
    }

    pub fn get_latency(&self) -> u32 {
        (self.pitch[0].get_latency() + self.pitch[1].get_latency()) / 2
    }

    pub fn reset(&mut self) {
        self.pitch[0].reset();
        self.pitch[1].reset();
    }

    pub fn process(&mut self, input: [f32; 2]) -> [f32; 2] {
        [self.pitch[0].process(input[0]), self.pitch[1].process(input[1])]
    }

}

impl Default for MyPitch {
    fn default() -> Self {
        Self {
            pitch: [
                PitchShifter::new(2, 100, 1, 0.0),
                PitchShifter::new(2, 100, 1, 0.0),
            ]
        }
    }
}