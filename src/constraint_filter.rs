/// Biquad filters with constraint-theory harmonic filtering.
use serde::{Deserialize, Serialize};

/// Biquad filter type.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FilterType {
    Lowpass,
    Highpass,
    Bandpass,
}

/// State-variable biquad filter using RBJ Audio EQ Cookbook coefficients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiquadFilter {
    pub filter_type: FilterType,
    pub cutoff: f64,
    pub q: f64,
    pub sample_rate: f64,
    // Coefficients
    #[serde(skip)]
    b0: f64,
    #[serde(skip)]
    b1: f64,
    #[serde(skip)]
    b2: f64,
    #[serde(skip)]
    a1: f64,
    #[serde(skip)]
    a2: f64,
    // State (Direct Form II Transposed)
    #[serde(skip)]
    z1: f64,
    #[serde(skip)]
    z2: f64,
    #[serde(skip)]
    computed: bool,
}

impl BiquadFilter {
    pub fn new(filter_type: FilterType, cutoff: f64, q: f64, sample_rate: f64) -> Self {
        let mut f = Self {
            filter_type,
            cutoff,
            q,
            sample_rate,
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
            computed: false,
        };
        f.compute_coefficients();
        f
    }

    fn compute_coefficients(&mut self) {
        let w0 = 2.0 * std::f64::consts::PI * self.cutoff / self.sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * self.q);

        let (b0, b1, b2, a0, a1, a2) = match self.filter_type {
            FilterType::Lowpass => {
                let b1 = 1.0 - cos_w0;
                let b0 = b1 / 2.0;
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Highpass => {
                let b0 = (1.0 + cos_w0) / 2.0;
                let b1 = -(1.0 + cos_w0);
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Bandpass => {
                // Constant skirt gain, peak gain = Q
                let b0 = self.q * alpha;
                let b1 = 0.0;
                let b2 = -b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        // Normalize by a0
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
        self.computed = true;
    }

    /// Process a single sample through the filter.
    pub fn process(&mut self, input: f64) -> f64 {
        if !self.computed {
            self.compute_coefficients();
        }

        let output = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * output + self.z2;
        self.z2 = self.b2 * input - self.a2 * output;
        output
    }

    /// Process a buffer of samples in-place.
    pub fn process_buffer(&mut self, buffer: &mut [f64]) {
        for sample in buffer.iter_mut() {
            *sample = self.process(*sample);
        }
    }

    /// Reset filter state (not coefficients).
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

/// Consonance intervals in semitones (from the 12-TET system).
const CONSONANT_INTERVALS: &[f64] = &[0.0, 3.0, 4.0, 5.0, 7.0, 9.0, 12.0];

/// A filter that emphasizes harmonically consonant intervals and attenuates dissonant ones.
///
/// Uses a series of narrow bandpass filters tuned to consonant harmonics of a root frequency.
#[derive(Debug, Clone)]
pub struct ConsonanceFilter {
    /// Root frequency for harmonic reference.
    pub root_freq: f64,
    /// Bandwidth of each consonant band in octaves.
    pub bandwidth: f64,
    /// Sample rate.
    pub sample_rate: f64,
    /// Blend: 1.0 = full consonance filtering, 0.0 = bypass.
    pub blend: f64,
    filters: Vec<BiquadFilter>,
}

impl ConsonanceFilter {
    pub fn new(root_freq: f64, sample_rate: f64, bandwidth: f64, blend: f64) -> Self {
        let mut filters = Vec::new();

        for &interval in CONSONANT_INTERVALS {
            let harmonic_freq = root_freq * (2.0_f64.powf(interval / 12.0));
            if harmonic_freq < sample_rate / 2.0 {
                let q = harmonic_freq / (bandwidth * root_freq).max(1.0);
                filters.push(BiquadFilter::new(
                    FilterType::Bandpass,
                    harmonic_freq,
                    q.max(0.1),
                    sample_rate,
                ));
            }
        }

        Self {
            root_freq,
            bandwidth,
            sample_rate,
            blend: blend.clamp(0.0, 1.0),
            filters,
        }
    }

    /// Process a buffer through the consonance filter.
    pub fn process_buffer(&mut self, buffer: &mut [f64]) {
        if self.blend < 0.001 {
            return;
        }

        let original = buffer.to_vec();

        // Zero buffer and accumulate filtered versions
        buffer.fill(0.0);

        for filter in &mut self.filters {
            let mut temp = original.clone();
            filter.process_buffer(&mut temp);
            for (out, &t) in buffer.iter_mut().zip(temp.iter()) {
                *out += t;
            }
        }

        // Normalize by filter count and blend with original
        let n = self.filters.len().max(1) as f64;
        for (out, &orig) in buffer.iter_mut().zip(original.iter()) {
            let filtered = *out / n;
            *out = orig * (1.0 - self.blend) + filtered * self.blend;
        }
    }

    /// Reset all internal filter states.
    pub fn reset(&mut self) {
        for f in &mut self.filters {
            f.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lowpass_dc_gain() {
        let mut f = BiquadFilter::new(FilterType::Lowpass, 1000.0, 0.707, 44100.0);
        let dc = 1.0;
        // After many iterations, DC should pass through LP at unity gain
        let mut last = 0.0;
        for _ in 0..1000 {
            last = f.process(dc);
        }
        assert!(
            (last - 1.0).abs() < 0.01,
            "LP DC gain should be ~1.0, got {last}"
        );
    }

    #[test]
    fn test_highpass_rejects_dc() {
        let mut f = BiquadFilter::new(FilterType::Highpass, 1000.0, 0.707, 44100.0);
        let mut last = 1.0;
        for _ in 0..1000 {
            last = f.process(1.0);
        }
        assert!(last.abs() < 0.01, "HP should reject DC, got {last}");
    }

    #[test]
    fn test_consonance_filter_doesnt_crash() {
        let mut cf = ConsonanceFilter::new(261.63, 44100.0, 0.5, 0.5);
        let mut buf: Vec<f64> = (0..1024).map(|i| (i as f64 * 0.1).sin()).collect();
        cf.process_buffer(&mut buf);
        // Should produce finite output
        for &s in &buf {
            assert!(s.is_finite(), "Output should be finite");
        }
    }
}
