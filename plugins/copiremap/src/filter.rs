use iir_filters::filter::{DirectForm2Transposed, Filter};
use iir_filters::filter_design::{butter, FilterType, ZPKCoeffs};
use iir_filters::sos::{Sos, zpk2sos};

pub struct MyFilter {
    zpk: ZPKCoeffs,
    sos: Sos,
    dft2: [DirectForm2Transposed; 2],
}

impl MyFilter {

    pub fn set_filter(&mut self, order: u8, filter: FilterType, sample_rate: f32) {
        self.zpk = butter(order as u32, filter, sample_rate).unwrap();
        self.sos = zpk2sos(&self.zpk, None).unwrap();
        self.dft2 = [DirectForm2Transposed::new(&self.sos), DirectForm2Transposed::new(&self.sos)];
    }

    pub fn process(&mut self, input: f32, audio_id: usize) -> f32 {
        self.dft2[audio_id].filter(input)
    }
}

impl Default for MyFilter {
    fn default() -> Self {
        let zpk = butter(1, FilterType::LowPass(10.0), 44100.0).unwrap();
        let sos = zpk2sos(&zpk, None).unwrap();
        let dft2 = [DirectForm2Transposed::new(&sos), DirectForm2Transposed::new(&sos)];
        Self {
            zpk,
            sos,
            dft2,
        }
    }
}