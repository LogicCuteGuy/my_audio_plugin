pub struct Delay {
    pub delay: u32,
    pub delay_samples: Vec<f32>,
}

impl Delay {

    pub fn new(delay: u32) -> Self {
        let mut delay_samples: Vec<f32> = Vec::new();
        for i in 0..delay {
            delay_samples.push(0.0);
        }
        Self {
            delay,
            delay_samples,
        }
    }

    pub fn set_delay(&mut self, delay: u32) {
        self.delay = delay;
        self.delay_samples.resize(delay as usize, 0.0);
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.delay_samples.push(input);
        let out = self.delay_samples.get(0).unwrap().clone();
        self.delay_samples.remove(0);
        out
    }

}