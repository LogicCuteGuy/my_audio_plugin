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
    }

    pub fn process(&mut self, input: [f32; 2]) -> [f32; 2] {
        [self.dft2[0].filter(input[0]), self.dft2[1].filter(input[1])]
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