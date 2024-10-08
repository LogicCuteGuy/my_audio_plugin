use crate::design::*;
use crate::kernel::*;
use nalgebra::{convert as _c, RealField as Real, Vector2 as Vec2};

/// A single filter band.
#[derive(Copy, Clone, Debug)]
pub struct Filter<R: Real> {
    kernel: Kernel<R>,
    design: Design<R>,
    sample_rate: R,
}

impl<R: Real + Copy> Filter<R> {
    /// Construct a new filter instance
    pub fn new(sample_rate: R) -> Self {
        let design = Design {
            curve: Curve::Peak,
            gain: _c(0.0),
            frequency: _c(0.1),
            resonance: _c(1.0),
        };
        let kernel = Kernel::new();
        let mut self_ = Self {
            design,
            kernel,
            sample_rate,
        };
        self_.update();
        self_
    }

    #[inline]
    pub fn set(&mut self, curve: Curve, frequency: R, resonance: R, gain: R, sample_rate: R) {
        self.design = Design {
            frequency: normalize_frequency(frequency, sample_rate),
            gain,
            resonance,
            curve,
        };
        self.sample_rate = sample_rate;
        self.update();
    }

    /// Get a copy of the filter's current design parameters.
    pub fn get_design(&self) -> Design<R> {
        self.design
    }

    /// Get a copy of the current filter state.
    #[inline]
    pub fn get_state(&self) -> Vec2<R> {
        self.kernel.s
    }

    /// Set the curve parameter (lowpass, highpass, bandpass, etc) of the filter.
    #[inline]
    pub fn set_curve(&mut self, curve: Curve) {
        self.design.curve = curve;
        self.update();
    }

    /// Set the critical frequency of the filter.
    #[inline]
    pub fn set_frequency(&mut self, freq_hz: R) {
        self.design.frequency = normalize_frequency(freq_hz, self.sample_rate);
        self.update();
    }

    /// set the gain of the filter. Meaningless for some filter curves.
    #[allow(non_snake_case)]
    #[inline]
    pub fn set_gain(&mut self, gain_dB: R) {
        self.design.gain = gain_dB;
        self.update();
    }

    /// Set the resonance (aka "Q" factor) of the filter
    #[inline]
    pub fn set_resonance(&mut self, resonance: R) {
        self.design.resonance = resonance;
        self.update();
    }

    /// Change the sample rate of the filter. This will reset the filter state.
    #[inline]
    pub fn set_sample_rate(&mut self, sample_rate: R) {
        self.sample_rate = sample_rate;
        self.update();
    }

    /// Zero the state of the filter.
    pub fn reset(&mut self) {
        self.kernel.reset();
    }

    #[inline]
    fn update(&mut self) {
        let (num, den) = self.design.digital_xfer_fn();
        self.kernel.set(num, den);
    }

    #[inline]
    pub fn filter(&mut self, x: R) -> R {
        self.kernel.eval(x)
    }

    #[inline]
    pub fn filter_buffer(&mut self, input: &mut [R]) {
        for x in input {
            *x = self.filter(*x);
        }
    }
}
