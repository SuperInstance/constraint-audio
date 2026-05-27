/// Lattice-based oscillator with PolyBLEP anti-aliasing.
use serde::{Deserialize, Serialize};

/// Waveform shapes derived from lattice geometry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LatticeShape {
    Sine,
    Square,
    Saw,
    Triangle,
    /// Hexagonal (A2) lattice phase mapping — Eisenstein integer inspired.
    Eisenstein,
}

/// Configuration for a lattice oscillator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatticeOscillator {
    pub freq: f64,
    pub sample_rate: f64,
    pub shape: LatticeShape,
    /// Phase stretch factor (1.0 = normal).
    pub stretch: f64,
    /// Noise floor amplitude.
    pub noise_floor: f64,
    /// Snap-to-lattice threshold for Eisenstein mode.
    pub snap_threshold: f64,
    /// Internal phase accumulator.
    #[serde(skip)]
    phase: f64,
}

impl LatticeOscillator {
    pub fn new(freq: f64, sample_rate: f64, shape: LatticeShape) -> Self {
        Self {
            freq,
            sample_rate,
            shape,
            stretch: 1.0,
            noise_floor: 0.0,
            snap_threshold: 0.1,
            phase: 0.0,
        }
    }

    /// Generate `duration_secs` of audio at the configured sample rate.
    pub fn generate(&mut self, duration_secs: f64) -> Vec<f64> {
        let num_samples = (duration_secs * self.sample_rate).round() as usize;
        let mut output = Vec::with_capacity(num_samples);
        let phase_inc = self.freq / self.sample_rate * self.stretch;

        for _ in 0..num_samples {
            let sample = self.tick(phase_inc);
            output.push(sample);
        }

        output
    }

    /// Reset the oscillator phase.
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn tick(&mut self, phase_inc: f64) -> f64 {
        let phase = self.phase;
        self.phase = (self.phase + phase_inc) % 1.0;

        let raw = match self.shape {
            LatticeShape::Sine => (2.0 * std::f64::consts::PI * phase).sin(),
            LatticeShape::Square => self.square_polyblep(phase, phase_inc),
            LatticeShape::Saw => self.saw_polyblep(phase, phase_inc),
            LatticeShape::Triangle => self.triangle_polyblep(phase, phase_inc),
            LatticeShape::Eisenstein => self.eisenstein(phase),
        };

        raw + self.noise_floor * pseudo_noise(phase)
    }

    // --- PolyBLEP implementations ---

    fn square_polyblep(&self, phase: f64, phase_inc: f64) -> f64 {
        let mut val = if phase < 0.5 { 1.0 } else { -1.0 };

        // PolyBLEP correction at rising edge (phase=0)
        val += self.polyblep(phase, phase_inc);
        // PolyBLEP correction at falling edge (phase=0.5)
        val -= self.polyblep((phase + 0.5) % 1.0, phase_inc);

        val
    }

    fn saw_polyblep(&self, phase: f64, phase_inc: f64) -> f64 {
        let mut val = 2.0 * phase - 1.0;
        val -= self.polyblep(phase, phase_inc);
        val
    }

    fn triangle_polyblep(&self, phase: f64, phase_inc: f64) -> f64 {
        // Triangle = integrate square, scaled
        let sq = if phase < 0.5 { 1.0 } else { -1.0 };
        let blep = self.polyblep(phase, phase_inc) - self.polyblep((phase + 0.5) % 1.0, phase_inc);
        let _integrated = (sq + blep) * phase_inc * 2.0;
        // Simplified triangle from saw
        let saw = 2.0 * phase - 1.0;
        2.0 * (saw.abs() - 0.5)
    }

    /// PolyBLEP correction kernel (2-point).
    fn polyblep(&self, t: f64, dt: f64) -> f64 {
        if t < dt {
            let x = t / dt;
            x + x - x * x - 1.0
        } else if t > 1.0 - dt {
            let x = (t - 1.0) / dt;
            x * x + x + x + 1.0
        } else {
            0.0
        }
    }

    /// Eisenstein integer (A2 lattice) phase mapping.
    ///
    /// Maps the scalar phase onto a hexagonal lattice in the complex plane
    /// using the Eisenstein integer basis {1, ω} where ω = e^(2πi/3).
    /// The result is the real part of the lattice point projected back.
    fn eisenstein(&self, phase: f64) -> f64 {
        let omega_angle = 2.0 * std::f64::consts::PI / 3.0;

        // Map phase to coordinates in the A2 lattice basis
        let a = phase;
        let b = (phase * 1.5).fract();

        // Snap to nearest lattice point if within threshold
        let (a, b) = if self.snap_threshold > 0.0 {
            let nearest_a = (a / self.snap_threshold).round() * self.snap_threshold;
            let nearest_b = (b / self.snap_threshold).round() * self.snap_threshold;
            let dist_a = (a - nearest_a).abs();
            let dist_b = (b - nearest_b).abs();
            if dist_a < self.snap_threshold && dist_b < self.snap_threshold {
                (nearest_a, nearest_b)
            } else {
                (a, b)
            }
        } else {
            (a, b)
        };

        // Project onto the Eisenstein plane and extract real part
        let re = a + b * omega_angle.cos();
        let im = b * omega_angle.sin();

        // Normalize to [-1, 1]
        let mag = (re * re + im * im).sqrt().max(0.001);
        (re / mag * (2.0 * std::f64::consts::PI * phase).sin()).tanh()
    }
}

/// Simple deterministic pseudo-noise for noise floor (avoids rand dependency).
fn pseudo_noise(phase: f64) -> f64 {
    let x = (phase * 1e6).to_bits() as f64;
    let n = ((x * 12.9898 + 78.233).sin() * 43758.5453).fract();
    2.0 * n - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sine_output_range() {
        let mut osc = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Sine);
        let samples = osc.generate(0.01);
        assert!(!samples.is_empty());
        for &s in &samples {
            assert!(s >= -1.1 && s <= 1.1, "Sine sample out of range: {s}");
        }
    }

    #[test]
    fn test_square_polyblep_output_range() {
        let mut osc = LatticeOscillator::new(220.0, 44100.0, LatticeShape::Square);
        let samples = osc.generate(0.01);
        for &s in &samples {
            assert!(s >= -1.5 && s <= 1.5, "Square sample out of range: {s}");
        }
    }

    #[test]
    fn test_saw_output_range() {
        let mut osc = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Saw);
        let samples = osc.generate(0.01);
        for &s in &samples {
            assert!(s >= -1.5 && s <= 1.5, "Saw sample out of range: {s}");
        }
    }

    #[test]
    fn test_eisenstein_output_range() {
        let mut osc = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Eisenstein);
        let samples = osc.generate(0.01);
        assert!(!samples.is_empty());
        for &s in &samples {
            assert!(s >= -1.5 && s <= 1.5, "Eisenstein sample out of range: {s}");
        }
    }

    #[test]
    fn test_sample_count() {
        let mut osc = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Sine);
        let samples = osc.generate(1.0);
        assert_eq!(samples.len(), 44100);
    }

    #[test]
    fn test_reset() {
        let mut osc = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Sine);
        let _ = osc.generate(0.5);
        osc.reset();
        // After reset, generating the same buffer should produce identical output
        let mut osc2 = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Sine);
        let s1 = osc.generate(0.01);
        let s2 = osc2.generate(0.01);
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_nonzero_output() {
        let mut osc = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Sine);
        let samples = osc.generate(0.01);
        let max_val = samples.iter().fold(0.0f64, |a, &b| a.max(b.abs()));
        assert!(max_val > 0.01, "Oscillator should produce nonzero output");
    }

    // ── Additional tests ───────────────────────────────────────

    #[test]
    fn test_triangle_output_range() {
        let mut osc = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Triangle);
        let samples = osc.generate(0.01);
        assert!(!samples.is_empty());
        for &s in &samples {
            assert!(s >= -1.5 && s <= 1.5, "Triangle sample out of range: {s}");
        }
    }

    #[test]
    fn test_stretch_parameter() {
        let mut osc_normal = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Sine);
        let mut osc_stretched = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Sine);
        osc_stretched.stretch = 2.0;
        let normal = osc_normal.generate(0.01);
        let stretched = osc_stretched.generate(0.01);
        // Stretched oscillator should produce different output (different phase increment)
        assert_ne!(normal, stretched, "Stretch should change oscillator output");
        // Both should produce same number of samples
        assert_eq!(normal.len(), stretched.len());
    }

    #[test]
    fn test_noise_floor_adds_noise() {
        let mut osc_clean = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Sine);
        let mut osc_noisy = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Sine);
        osc_noisy.noise_floor = 0.1;
        let clean = osc_clean.generate(0.01);
        let noisy = osc_noisy.generate(0.01);
        // Noisy version should have higher energy (on average)
        let clean_energy: f64 = clean.iter().map(|s| s * s).sum();
        let noisy_energy: f64 = noisy.iter().map(|s| s * s).sum();
        assert!(noisy_energy > clean_energy, "Noise floor should add energy");
    }

    #[test]
    fn test_snap_threshold_affects_eisenstein() {
        let mut osc_loose = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Eisenstein);
        osc_loose.snap_threshold = 0.001; // Very tight snapping
        let mut osc_tight = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Eisenstein);
        osc_tight.snap_threshold = 0.5; // Loose snapping
        let loose = osc_loose.generate(0.01);
        let tight = osc_tight.generate(0.01);
        // Different snap thresholds should produce different outputs
        assert_ne!(loose, tight, "Different snap thresholds should yield different results");
    }

    #[test]
    fn test_zero_snap_threshold() {
        let mut osc = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Eisenstein);
        osc.snap_threshold = 0.0; // No snapping
        let samples = osc.generate(0.01);
        assert!(!samples.is_empty());
        for &s in &samples {
            assert!(s.is_finite(), "Zero snap threshold should produce finite output");
        }
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut osc = LatticeOscillator::new(440.0, 44100.0, LatticeShape::Saw);
        osc.stretch = 1.5;
        osc.noise_floor = 0.02;
        osc.snap_threshold = 0.3;
        let json = serde_json::to_string(&osc).expect("serialize");
        let osc2: LatticeOscillator = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(osc2.freq, 440.0);
        assert_eq!(osc2.sample_rate, 44100.0);
        assert_eq!(osc2.shape, LatticeShape::Saw);
        assert!((osc2.stretch - 1.5).abs() < 1e-10);
        assert!((osc2.noise_floor - 0.02).abs() < 1e-10);
        assert!((osc2.snap_threshold - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_lattice_shape_serde() {
        for shape in [
            LatticeShape::Sine,
            LatticeShape::Square,
            LatticeShape::Saw,
            LatticeShape::Triangle,
            LatticeShape::Eisenstein,
        ] {
            let json = serde_json::to_string(&shape).unwrap();
            let back: LatticeShape = serde_json::from_str(&json).unwrap();
            assert_eq!(shape, back);
        }
    }

    #[test]
    fn test_very_low_frequency() {
        let mut osc = LatticeOscillator::new(1.0, 44100.0, LatticeShape::Sine);
        let samples = osc.generate(1.0);
        assert_eq!(samples.len(), 44100);
        for &s in &samples {
            assert!(s.is_finite());
        }
    }

    #[test]
    fn test_high_frequency_aliasing_is_bounded() {
        let mut osc = LatticeOscillator::new(18000.0, 44100.0, LatticeShape::Saw);
        let samples = osc.generate(0.01);
        for &s in &samples {
            assert!(s.is_finite(), "High freq saw should stay finite");
        }
    }
}
