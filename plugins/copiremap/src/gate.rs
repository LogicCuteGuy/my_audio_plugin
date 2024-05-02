use nih_plug::audio_setup::BufferConfig;

pub struct MyGate {
    pub fast: [f32; 2],
    sum: [f32; 2],
    pub param: [f32; 2],
    count: [u16; 2]
}

impl MyGate {
    pub fn new() -> Self {
        Self {
            fast: [0.0; 2],
            sum: [0.0; 2],
            param: [0.0; 2],
            count: [0; 2]
        }
    }

    pub fn update_fast_param(&mut self, sample: f32, buffer_config: &BufferConfig, threshold: f32, attack_ms: f32, release_ms: f32, buf_size: usize, flip: bool, audio_id: usize) -> (bool, bool) {
        self.sum[audio_id] += sample * sample;
        let delta_attack = (1.0 / (attack_ms * 0.001 * buffer_config.sample_rate * buf_size as f32)).min(1.0); // Change per sample for attack
        let delta_release = (1.0 / (release_ms * 0.001 * buffer_config.sample_rate * buf_size as f32)).min(1.0); // Change per sample for release
        self.count[audio_id] += 1;
        if self.count[audio_id] > buf_size as u16 {
            self.count[audio_id] = 0;
            self.fast[audio_id] = (self.sum[audio_id] / buf_size as f32).sqrt();
            self.sum[audio_id] = 0.0;
        }
        if self.fast[audio_id] >= threshold && self.param[audio_id] >= 1.0 {
            (!flip, flip)
        } else if self.fast[audio_id] >= threshold{
            self.param[audio_id] += delta_attack; // Increase param for attack
            (true, true)
        } else if self.fast[audio_id] < threshold && self.param[audio_id] <= 0.0 {
            (flip, !flip)
        } else {
            self.param[audio_id] -= delta_release; // Decrease param for release
            (true, true)
        }
    }

    pub fn get_param(&self, flip: bool, audio_id: usize) -> f32 {
        if flip {1.0 - self.param[audio_id]} else {self.param[audio_id]}
    }

    pub fn get_param_inv(&self, flip: bool, audio_id: usize) -> f32 {
        if flip {self.param[audio_id]} else {1.0 - self.param[audio_id]}
    }
}