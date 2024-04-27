use nih_plug::audio_setup::BufferConfig;

pub struct MyGate {
    pub fast: f32,
    sum: f32,
    param: f32,
    count: u16
}

impl MyGate {
    pub fn new() -> Self {
        Self {
            fast: 0.0,
            sum: 0.0,
            param: 0.0,
            count: 0,
        }
    }

    pub fn update_fast_param(&mut self, sample: f32, buffer_config: &BufferConfig, threshold: f32, attack_ms: f32, release_ms: f32, buf_size: usize) -> (bool, bool) {
        self.sum += sample * sample;
        let delta_attack = (1.0 / (attack_ms * 0.001 * buffer_config.sample_rate * buf_size as f32)).min(1.0); // Change per sample for attack
        let delta_release = (1.0 / (release_ms * 0.001 * buffer_config.sample_rate * buf_size as f32)).min(1.0); // Change per sample for release
        self.count += 1;
        if self.count > buf_size as u16 {
            self.count = 0;
            self.fast = (self.sum / buf_size as f32).sqrt();
            self.sum = 0.0;
        }
        if self.fast >= threshold && self.param >= 1.0 {
            (true, false)
        } else if self.fast >= threshold {
            self.param += delta_attack / 2.0; // Increase param for attack
            (true, true)
        } else if self.fast < threshold && self.param <= 0.0 {
            (false, true)
        } else {
            self.param -= delta_release / 2.0; // Decrease param for release
            (true, true)
        }
    }

    pub fn get_param(&self) -> f32 {
        self.param
    }

    pub fn get_param_inv(&self) -> f32 {
        1.0 - self.param
    }
}