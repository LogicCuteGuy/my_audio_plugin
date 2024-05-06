// Spectral Compressor: an FFT based compressor
// Copyright (C) 2021-2024 Robbert van der Helm
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use nih_plug::prelude::*;
use realfft::num_complex::Complex32;
use crate::GlobalParams;

// These are the parameter name prefixes used for the downwards and upwards compression parameters.
// The ID prefixes a re set in the `CompressorBankParams` struct.
const DOWNWARDS_NAME_PREFIX: &str = "Downwards";
const UPWARDS_NAME_PREFIX: &str = "Upwards";

/// The envelopes are initialized to the RMS value of a -24 dB sine wave to make sure extreme upwards
/// compression doesn't cause pops when switching between window sizes and when deactivating and
/// reactivating the plugin.
const ENVELOPE_INIT_VALUE: f32 = std::f32::consts::FRAC_1_SQRT_2 / 8.0;

/// The target frequency for the high frequency ratio rolloff. This is fixed to prevent Spectral
/// Compressor from getting brighter as the sample rate increases.
#[allow(unused)]
const HIGH_FREQ_RATIO_ROLLOFF_FREQUENCY: f32 = 22_050.0;
const HIGH_FREQ_RATIO_ROLLOFF_FREQUENCY_LN: f32 = 10.001068; // 22_050.0f32.ln()

/// The length of time over which the envelope followers fade back from being instant to using the
/// configured timingsafter the compressor bank has been reset.
const ENVELOPE_FOLLOWER_TIMING_FADE_MS: f32 = 150.0;

pub struct StarsBank {
    /// For each compressor bin, `ln(freq)` where `freq` is the frequency associated with that
    /// compressor. This is precomputed since all update functions need it.
    ln_freqs: Vec<f32>,

    /// The current envelope value for this bin, in linear space. Indexed by
    /// `[channel_idx][compressor_idx]`.
    envelopes: Vec<Vec<f32>>,
    /// A scaling factor for the envelope follower timings. This is set to 0 and then slowly brought
    /// back up to 1 after after [`StarsBank::reset()`] has been called to allow the envelope
    /// followers to settle back in.
    envelope_followers_timing_scale: f32,
    /// When sidechaining is enabled, this contains the per-channel frqeuency spectrum magnitudes
    /// for the current block. The compressor thresholds and knee values are multiplied by these
    /// values to get the effective thresholds.
    sidechain_spectrum_magnitudes: Vec<Vec<f32>>,
    /// The window size this compressor bank was configured for. This is used to compute the
    /// coefficients for the envelope followers in the process function.
    window_size: usize,
    /// The sample rate this compressor bank was configured for. This is used to compute the
    /// coefficients for the envelope followers in the process function.
    sample_rate: f32,

}

impl StarsBank {
    /// Set up the compressor for the given channel count and maximum FFT window size. The
    /// compressors won't be initialized yet.
    pub fn new(
        num_channels: usize,
        max_window_size: usize,
    ) -> Self {
        let complex_buffer_len = max_window_size / 2 + 1;

        StarsBank {
            ln_freqs: Vec::with_capacity(complex_buffer_len),

            envelopes: vec![Vec::with_capacity(complex_buffer_len); num_channels],
            envelope_followers_timing_scale: 0.0,
            sidechain_spectrum_magnitudes: vec![
                Vec::with_capacity(complex_buffer_len);
                num_channels
            ],
            window_size: 0,
            sample_rate: 1.0,
        }
    }

    /// Change the capacities of the internal buffers to fit new parameters. Use the
    /// `.reset_for_size()` method to clear the buffers and set the current window size.
    pub fn update_capacity(&mut self, num_channels: usize, max_window_size: usize) {
        let complex_buffer_len = max_window_size / 2 + 1;

        self.ln_freqs
            .reserve_exact(complex_buffer_len.saturating_sub(self.ln_freqs.len()));

        self.envelopes.resize_with(num_channels, Vec::new);
        for envelopes in self.envelopes.iter_mut() {
            envelopes.reserve_exact(complex_buffer_len.saturating_sub(envelopes.len()));
        }

        self.sidechain_spectrum_magnitudes
            .resize_with(num_channels, Vec::new);
        for magnitudes in self.sidechain_spectrum_magnitudes.iter_mut() {
            magnitudes.reserve_exact(complex_buffer_len.saturating_sub(magnitudes.len()));
        }
    }

    /// Resize the number of compressors to match the current window size. Also precomputes the
    /// 2-log frequencies for each bin.
    ///
    /// If the window size is larger than the maximum window size, then this will allocate.
    pub fn resize(&mut self, buffer_config: &BufferConfig, window_size: usize) {
        let complex_buffer_len = window_size / 2 + 1;

        // These 2-log frequencies are needed when updating the compressor parameters, so we'll just
        // precompute them to avoid having to repeat the same expensive computations all the time
        self.ln_freqs.resize(complex_buffer_len, 0.0);
        // The first one should always stay at zero, `0.0f32.ln() == NaN`.
        for (i, ln_freq) in self.ln_freqs.iter_mut().enumerate().skip(1) {
            let freq = (i as f32 / window_size as f32) * buffer_config.sample_rate;
            *ln_freq = freq.ln();
        }

        for envelopes in self.envelopes.iter_mut() {
            envelopes.resize(complex_buffer_len, ENVELOPE_INIT_VALUE);
        }

        for magnitudes in self.sidechain_spectrum_magnitudes.iter_mut() {
            magnitudes.resize(complex_buffer_len, 0.0);
        }

        self.window_size = window_size;
        self.sample_rate = buffer_config.sample_rate;
    }

    /// Clear out the envelope followers.
    pub fn reset(&mut self) {
        // This will make the timings instant for the first iteration after a reset and then slowly
        // fade the timings back to their intended values so the envelope followers can settle in.
        // Otherwise suspending and resetting the plugin, or changing the window size, may result in
        // some huge spikes.
        self.envelope_followers_timing_scale = 0.0;

        // Sidechain data doesn't need to be reset as it will be overwritten immediately before use
    }

    /// Apply the magnitude compression to a buffer of FFT bins. The compressors are first updated
    /// if needed. The overlap amount is needed to compute the effective sample rate. The
    /// `first_non_dc_bin` argument is used to avoid upwards compression on the DC bins, or the
    /// neighbouring bins the DC signal may have been convolved into because of the Hann window
    /// function.
    pub fn process(
        &mut self,
        buffer: &mut [Complex32],
        channel_idx: usize,
        params: &GlobalParams,
        overlap_times: usize,
        first_non_dc_bin: usize,
    ) {
        nih_debug_assert_eq!(buffer.len(), self.ln_freqs.len());
        
        match params.morph.value() {
            false => {
                self.update_envelopes(buffer, channel_idx, params, overlap_times);
                self.compress(buffer, channel_idx, params, first_non_dc_bin)
            }
            true => {
                self.update_envelopes(buffer, channel_idx, params, overlap_times);
                self.compress_sidechain_match(buffer, channel_idx, params, first_non_dc_bin)
            }
        };

    }

    /// Set the sidechain frequency spectrum magnitudes just before a [`process()`][Self::process()]
    /// call. These will be multiplied with the existing compressor thresholds and knee values to
    /// get the effective values for use with sidechaining.
    pub fn process_sidechain(&mut self, sc_buffer: &[Complex32], channel_idx: usize) {
        nih_debug_assert_eq!(sc_buffer.len(), self.ln_freqs.len());

        self.update_sidechain_spectra(sc_buffer, channel_idx);
    }

    /// Update the envelope followers based on the bin magnitudes.
    fn update_envelopes(
        &mut self,
        buffer: &[Complex32],
        channel_idx: usize,
        params: &GlobalParams,
        overlap_times: usize,
    ) {
        let effective_sample_rate =
            self.sample_rate / (self.window_size as f32 / overlap_times as f32);

        // The timings are scaled by `self.envelope_followers_timing_scale` to allow the envelope
        // followers to settle in quicker after a reset
        let attack_ms =
            params.compressor_attack_ms.value() * self.envelope_followers_timing_scale;
        let release_ms =
            params.compressor_release_ms.value() * self.envelope_followers_timing_scale;

        // This needs to gradually fade from 0.0 back to 1.0 after a reset
        if self.envelope_followers_timing_scale < 1.0 && channel_idx == self.envelopes.len() - 1 {
            let delta =
                ((ENVELOPE_FOLLOWER_TIMING_FADE_MS / 1000.0) * effective_sample_rate).recip();
            self.envelope_followers_timing_scale =
                (self.envelope_followers_timing_scale + delta).min(1.0);
        }

        // The coefficient the old envelope value is multiplied by when the current rectified sample
        // value is above the envelope's value. The 0 to 1 step response retains 36.8% of the old
        // value after the attack time has elapsed, and current value is 63.2% of the way towards 1.
        // The effective sample rate needs to compensate for the periodic nature of the STFT
        // operation. Since with a 2048 sample window and 4x overlap, you'd run this function once
        // for every 512 samples.
        let attack_old_t = if attack_ms == 0.0 {
            0.0
        } else {
            (-1.0 / (attack_ms / 1000.0 * effective_sample_rate)).exp()
        };
        let attack_new_t = 1.0 - attack_old_t;
        // The same as `attack_old_t`, but for the release phase of the envelope follower
        let release_old_t = if release_ms == 0.0 {
            0.0
        } else {
            (-1.0 / (release_ms / 1000.0 * effective_sample_rate)).exp()
        };
        let release_new_t = 1.0 - release_old_t;

        for (bin, envelope) in buffer.iter().zip(self.envelopes[channel_idx].iter_mut()) {
            let magnitude = bin.norm();
            if *envelope > magnitude {
                // Release stage
                *envelope = (release_old_t * *envelope) + (release_new_t * magnitude);
            } else {
                // Attack stage
                *envelope = (attack_old_t * *envelope) + (attack_new_t * magnitude);
            }
        }
    }

    /// The same as [`update_envelopes()`][Self::update_envelopes()], but based on the previously
    /// set sidechain bin magnitudes. This allows for channel linking.
    /// [`process_sidechain()`][Self::process_sidechain()] needs to be called for all channels
    /// before this function can be used to set the magnitude spectra.
    fn update_envelopes_sidechain(
        &mut self,
        channel_idx: usize,
        params: &GlobalParams,
        overlap_times: usize,
    ) {
        let effective_sample_rate =
            self.sample_rate / (self.window_size as f32 / overlap_times as f32);

        // The timings are scaled by `self.envelope_followers_timing_scale` to allow the envelope
        // followers to settle in quicker after a reset
        let attack_ms =
            params.compressor_attack_ms.value() * self.envelope_followers_timing_scale;
        let release_ms =
            params.compressor_release_ms.value() * self.envelope_followers_timing_scale;

        // This needs to gradually fade from 0.0 back to 1.0 after a reset
        if self.envelope_followers_timing_scale < 1.0 && channel_idx == self.envelopes.len() - 1 {
            let delta =
                ((ENVELOPE_FOLLOWER_TIMING_FADE_MS / 1000.0) * effective_sample_rate).recip();
            self.envelope_followers_timing_scale =
                (self.envelope_followers_timing_scale + delta).min(1.0);
        }

        // See `update_envelopes()`
        let attack_old_t = if attack_ms == 0.0 {
            0.0
        } else {
            (-1.0 / (attack_ms / 1000.0 * effective_sample_rate)).exp()
        };
        let attack_new_t = 1.0 - attack_old_t;
        let release_old_t = if release_ms == 0.0 {
            0.0
        } else {
            (-1.0 / (release_ms / 1000.0 * effective_sample_rate)).exp()
        };
        let release_new_t = 1.0 - release_old_t;

        // For the channel linking
        let num_channels = self.sidechain_spectrum_magnitudes.len() as f32;
        let other_channels_t = 1.0 / num_channels;
        let this_channel_t = 1.0 - (other_channels_t * (num_channels - 1.0));

        for (bin_idx, envelope) in self.envelopes[channel_idx].iter_mut().enumerate() {
            // In this mode the envelopes are set based on the sidechain signal, taking channel
            // linking into account
            let sidechain_magnitude: f32 = self
                .sidechain_spectrum_magnitudes
                .iter()
                .enumerate()
                .map(|(sidechain_channel_idx, magnitudes)| {
                    let t = if sidechain_channel_idx == channel_idx {
                        this_channel_t
                    } else {
                        other_channels_t
                    };

                    unsafe { magnitudes.get_unchecked(bin_idx) * t }
                })
                .sum::<f32>();

            if *envelope > sidechain_magnitude {
                // Release stage
                *envelope = (release_old_t * *envelope) + (release_new_t * sidechain_magnitude);
            } else {
                // Attack stage
                *envelope = (attack_old_t * *envelope) + (attack_new_t * sidechain_magnitude);
            }
        }
    }

    /// Update the spectral data using the sidechain input
    fn update_sidechain_spectra(&mut self, sc_buffer: &[Complex32], channel_idx: usize) {
        nih_debug_assert!(channel_idx < self.sidechain_spectrum_magnitudes.len());

        for (bin, magnitude) in sc_buffer
            .iter()
            .zip(self.sidechain_spectrum_magnitudes[channel_idx].iter_mut())
        {
            *magnitude = bin.norm();
        }
    }

    /// Actually do the thing. [`Self::update_envelopes()`] or
    /// [`Self::update_envelopes_sidechain()`] must have been called before calling this.
    ///
    /// # Panics
    ///
    /// Panics if the buffer does not have the same length as the one that was passed to the last
    /// `resize()` call.
    fn compress(
        &mut self,
        buffer: &mut [Complex32],
        channel_idx: usize,
        params: &GlobalParams,
        first_non_dc_bin: usize,
    ) {
        // NOTE: In the sidechain compression mode these envelopes are computed from the sidechain
        //       signal instead of the main input
        for (bin_idx, (bin, envelope)) in buffer
            .iter_mut()
            .zip(self.envelopes[channel_idx].iter())
            .enumerate()
        {
            // We'll apply the transfer curve to the envelope signal, and then scale the complex
            // `bin` by the gain difference
            let envelope_db = util::gain_to_db_fast_epsilon(*envelope);
            

            // If the comprssed output is -10 dBFS and the envelope follower was at -6 dBFS, then we
            // want to apply -4 dB of gain to the bin
            let gain_difference_db = envelope_db * 2.0;

            *bin *= util::db_to_gain_fast(gain_difference_db);
        }
    }

    /// The same as [`compress()`][Self::compress()], but multiplying the threshold and knee values
    /// with the sidechain gains.
    ///
    /// # Panics
    ///
    /// Panics if the buffer does not have the same length as the one that was passed to the last
    /// `resize()` call.
    fn compress_sidechain_match(
        &mut self,
        buffer: &mut [Complex32],
        channel_idx: usize,
        params: &GlobalParams,
        first_non_dc_bin: usize,
    ) {

        // For the channel linking
        let num_channels = self.sidechain_spectrum_magnitudes.len() as f32;
        let other_channels_t = 1.0 / num_channels;
        let this_channel_t = 1.0 - (other_channels_t * (num_channels - 1.0));
        
        for (bin_idx, (bin, envelope)) in buffer
            .iter_mut()
            .zip(self.envelopes[channel_idx].iter())
            .enumerate()
        {
            let envelope_db = util::gain_to_db_fast_epsilon(*envelope);

            // The idea here is that we scale the compressor thresholds/knee values by the sidechain
            // signal, thus sort of creating a dynamic multiband compressor
            let sidechain_scale: f32 = self
                .sidechain_spectrum_magnitudes
                .iter()
                .enumerate()
                .map(|(sidechain_channel_idx, magnitudes)| {
                    let t = if sidechain_channel_idx == channel_idx {
                        this_channel_t
                    } else {
                        other_channels_t
                    };

                    unsafe { magnitudes.get_unchecked(bin_idx) * t }
                })
                .sum::<f32>()
                // The thresholds may never reach zero as they are used in divisions
                .max(f32::EPSILON);
            let sidechain_scale_db = util::gain_to_db_fast_epsilon(sidechain_scale);
            

            // If the comprssed output is -10 dBFS and the envelope follower was at -6 dBFS, then we
            // want to apply -4 dB of gain to the bin
            let gain_difference_db = envelope_db * 2.0;

            *bin *= util::db_to_gain_fast(gain_difference_db);
        }
    }
}