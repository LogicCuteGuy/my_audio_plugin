

pub struct MyGate {
    fast: f32,
    sum: f32,
    param: f32,
}

impl MyGate {
    pub fn new() -> Self {
        Self {
            fast: 0.0,
            sum: 0.0,
            param: 0.0,
        }
    }

    pub fn update_fast_param(&mut self, sample: f32, param: f32) -> (bool, bool) {
        self.sum += sample * sample;
        if self.fast >= param && self.param >= 1.0 {
            (true, false)
        } else if self.fast >= param {
            self.param += 0.001;
            (true, true)
        } else if self.fast < param && self.param <= 0.0 {
            (false, true)
        } else {
            self.param -= 0.001;
            (true, true)
        }
    }

    pub fn get_param(&self) -> f32 {
        self.param
    }

    pub fn get_param_inv(&self) -> f32 {
        1.0 - self.param
    }

    pub fn update_fast(&mut self, samples: usize) {
        self.fast = (self.sum / samples as f32).sqrt();
        self.sum = 0.0;
    }
}