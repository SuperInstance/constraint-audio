/// Constraint-based synthesizer combining lattice oscillator, ADSR envelope, and filters.
use crate::constraint_filter::{BiquadFilter, ConsonanceFilter, FilterType};
use crate::lattice_oscillator::{LatticeOscillator, LatticeShape};
use serde::{Deserialize, Serialize};

/// ADSR envelope parameters (FunnelEnvelope equivalent).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdsrEnvelope {
    pub attack: f64,
    pub decay: f64,
    pub sustain: f64,
    pub release: f64,
}

impl Default for AdsrEnvelope {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.2,
        }
    }
}

impl AdsrEnvelope {
    /// Generate an envelope buffer for a note of the given duration.
    ///
    /// The total envelope length is `duration + release`. Attack ramps 0→1,
    /// decay ramps 1→sustain, sustain holds, release ramps sustain→0.
    pub fn generate(&self, duration: f64, sample_rate: f64) -> Vec<f64> {
        let total_duration = duration + self.release;
        let num_samples = (total_duration * sample_rate).round() as usize;
        let mut env = Vec::with_capacity(num_samples);

        let attack_samples = (self.attack * sample_rate).round() as usize;
        let decay_samples = (self.decay * sample_rate).round() as usize;
        let release_samples = (self.release * sample_rate).round() as usize;

        for i in 0..num_samples {
            let val = if i < attack_samples {
                // Attack phase
                i as f64 / attack_samples.max(1) as f64
            } else if i < attack_samples + decay_samples {
                // Decay phase
                let t = (i - attack_samples) as f64 / decay_samples.max(1) as f64;
                1.0 - t * (1.0 - self.sustain)
            } else if i < num_samples - release_samples {
                // Sustain phase
                self.sustain
            } else {
                // Release phase
                let t =
                    (i - (num_samples - release_samples)) as f64 / release_samples.max(1) as f64;
                self.sustain * (1.0 - t)
            };
            env.push(val.clamp(0.0, 1.0));
        }

        env
    }
}

/// Synth preset definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthPreset {
    pub name: String,
    pub shape: LatticeShape,
    pub envelope: AdsrEnvelope,
    pub filter_type: FilterType,
    pub filter_cutoff: f64,
    pub filter_q: f64,
    pub stretch: f64,
    pub noise_floor: f64,
}

/// Built-in presets matching the Python constraint-synth version.
pub fn builtin_presets() -> Vec<SynthPreset> {
    vec![
        SynthPreset {
            name: "EisensteinBass".into(),
            shape: LatticeShape::Eisenstein,
            envelope: AdsrEnvelope {
                attack: 0.005,
                decay: 0.15,
                sustain: 0.5,
                release: 0.1,
            },
            filter_type: FilterType::Lowpass,
            filter_cutoff: 800.0,
            filter_q: 2.0,
            stretch: 1.0,
            noise_floor: 0.0,
        },
        SynthPreset {
            name: "LatticeLead".into(),
            shape: LatticeShape::Saw,
            envelope: AdsrEnvelope {
                attack: 0.01,
                decay: 0.1,
                sustain: 0.8,
                release: 0.15,
            },
            filter_type: FilterType::Lowpass,
            filter_cutoff: 3000.0,
            filter_q: 1.5,
            stretch: 1.0,
            noise_floor: 0.005,
        },
        SynthPreset {
            name: "HarmonicBell".into(),
            shape: LatticeShape::Triangle,
            envelope: AdsrEnvelope {
                attack: 0.001,
                decay: 0.3,
                sustain: 0.0,
                release: 0.8,
            },
            filter_type: FilterType::Bandpass,
            filter_cutoff: 2000.0,
            filter_q: 3.0,
            stretch: 1.0,
            noise_floor: 0.0,
        },
        SynthPreset {
            name: "ConsonancePad".into(),
            shape: LatticeShape::Sine,
            envelope: AdsrEnvelope {
                attack: 0.3,
                decay: 0.2,
                sustain: 0.9,
                release: 0.5,
            },
            filter_type: FilterType::Lowpass,
            filter_cutoff: 1500.0,
            filter_q: 0.707,
            stretch: 1.0,
            noise_floor: 0.0,
        },
        SynthPreset {
            name: "GlitchLattice".into(),
            shape: LatticeShape::Square,
            envelope: AdsrEnvelope {
                attack: 0.0,
                decay: 0.05,
                sustain: 0.3,
                release: 0.02,
            },
            filter_type: FilterType::Highpass,
            filter_cutoff: 500.0,
            filter_q: 5.0,
            stretch: 1.3,
            noise_floor: 0.02,
        },
    ]
}

/// The main constraint-theory synthesizer.
pub struct ConstraintSynth {
    pub sample_rate: f64,
    pub preset: SynthPreset,
    pub consonance_blend: f64,
}

impl ConstraintSynth {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            preset: builtin_presets()[0].clone(),
            consonance_blend: 0.0,
        }
    }

    pub fn with_preset(sample_rate: f64, preset: SynthPreset) -> Self {
        Self {
            sample_rate,
            preset,
            consonance_blend: 0.0,
        }
    }

    /// Convert MIDI pitch (0-127) to frequency in Hz (A440 tuning).
    pub fn midi_to_freq(pitch: u8) -> f64 {
        440.0 * 2.0_f64.powf((pitch as f64 - 69.0) / 12.0)
    }

    /// Render a note as an audio buffer.
    ///
    /// Returns the buffer with ADSR envelope, oscillator, and filter applied.
    /// Buffer length is `duration + envelope.release` seconds.
    pub fn play_note(&mut self, pitch: u8, velocity: u8, duration: f64) -> Vec<f64> {
        let freq = Self::midi_to_freq(pitch);
        let amp = velocity as f64 / 127.0;

        // Generate ADSR envelope
        let envelope = self.preset.envelope.generate(duration, self.sample_rate);

        // Generate oscillator output
        let total_duration = duration + self.preset.envelope.release;
        let mut osc = LatticeOscillator::new(freq, self.sample_rate, self.preset.shape);
        osc.stretch = self.preset.stretch;
        osc.noise_floor = self.preset.noise_floor;
        let mut buffer = osc.generate(total_duration);

        // Apply envelope
        for (s, &e) in buffer.iter_mut().zip(envelope.iter()) {
            *s *= e * amp;
        }

        // Apply biquad filter
        let mut filter = BiquadFilter::new(
            self.preset.filter_type,
            self.preset.filter_cutoff,
            self.preset.filter_q,
            self.sample_rate,
        );
        filter.process_buffer(&mut buffer);

        // Optionally apply consonance filter
        if self.consonance_blend > 0.001 {
            let mut cf = ConsonanceFilter::new(freq, self.sample_rate, 0.5, self.consonance_blend);
            cf.process_buffer(&mut buffer);
        }

        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_to_freq() {
        // A4 = MIDI 69 = 440 Hz
        assert!((ConstraintSynth::midi_to_freq(69) - 440.0).abs() < 0.01);
        // C4 = MIDI 60 ≈ 261.63 Hz
        assert!((ConstraintSynth::midi_to_freq(60) - 261.63).abs() < 0.1);
    }

    #[test]
    fn test_play_note_length() {
        let preset = builtin_presets()[0].clone();
        let mut synth = ConstraintSynth::with_preset(44100.0, preset);
        let buffer = synth.play_note(60, 100, 0.5);
        // Duration 0.5 + release 0.1 = 0.6 seconds
        let expected = (0.6_f64 * 44100.0).round() as usize;
        assert_eq!(buffer.len(), expected);
    }

    #[test]
    fn test_play_note_finite() {
        let preset = builtin_presets()[0].clone();
        let mut synth = ConstraintSynth::with_preset(44100.0, preset);
        let buffer = synth.play_note(60, 100, 0.5);
        for &s in &buffer {
            assert!(s.is_finite(), "Output must be finite");
        }
    }

    #[test]
    fn test_play_note_velocity_zero() {
        let preset = builtin_presets()[0].clone();
        let mut synth = ConstraintSynth::with_preset(44100.0, preset);
        let buffer = synth.play_note(60, 0, 0.5);
        for &s in &buffer {
            assert!(s.abs() < 1e-10, "Velocity 0 should produce silence");
        }
    }

    #[test]
    fn test_all_presets_compile() {
        for preset in builtin_presets() {
            let mut synth = ConstraintSynth::with_preset(44100.0, preset);
            let buffer = synth.play_note(64, 80, 0.3);
            assert!(buffer.iter().all(|s| s.is_finite()));
        }
    }

    #[test]
    fn test_adsr_envelope_shape() {
        let env = AdsrEnvelope {
            attack: 0.1,
            decay: 0.1,
            sustain: 0.5,
            release: 0.1,
        };
        let buf = env.generate(0.3, 1000.0); // 300 samples note + 100 release
        assert_eq!(buf.len(), 400);
        // First sample should be 0 or near 0
        assert!(buf[0] < 0.01);
        // Peak should be during attack
        let max = buf.iter().fold(0.0f64, |a, &b| a.max(b));
        assert!(max > 0.99, "Peak should be near 1.0, got {max}");
        // Last sample should be 0
        assert!(buf.last().unwrap().abs() < 0.01);
    }

    // ── Additional tests ───────────────────────────────────────

    #[test]
    fn test_adsr_zero_attack() {
        let env = AdsrEnvelope {
            attack: 0.0,
            decay: 0.1,
            sustain: 0.7,
            release: 0.1,
        };
        let buf = env.generate(0.2, 1000.0);
        // With zero attack, first sample should already be at peak or decay
        assert!(buf[0] >= 0.99, "Zero attack should start at peak, got {}", buf[0]);
    }

    #[test]
    fn test_adsr_zero_decay() {
        let env = AdsrEnvelope {
            attack: 0.01,
            decay: 0.0,
            sustain: 0.8,
            release: 0.1,
        };
        let buf = env.generate(0.2, 1000.0);
        // Should produce valid output without division issues
        assert!(!buf.is_empty());
        for &s in &buf {
            assert!(s >= 0.0 && s <= 1.0, "ADSR value out of range: {s}");
        }
    }

    #[test]
    fn test_adsr_zero_release() {
        let env = AdsrEnvelope {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.5,
            release: 0.0,
        };
        let buf = env.generate(0.2, 1000.0);
        // Without release, total length should just be the note duration
        let expected: usize = (0.2_f64 * 1000.0).round() as usize;
        assert_eq!(buf.len(), expected);
    }

    #[test]
    fn test_adsr_default() {
        let env = AdsrEnvelope::default();
        assert!((env.attack - 0.01).abs() < 1e-10);
        assert!((env.decay - 0.1).abs() < 1e-10);
        assert!((env.sustain - 0.7).abs() < 1e-10);
        assert!((env.release - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_adsr_sustain_level() {
        let env = AdsrEnvelope {
            attack: 0.01,
            decay: 0.01,
            sustain: 0.3,
            release: 0.01,
        };
        let buf = env.generate(0.5, 1000.0); // long sustain
        // Middle of buffer should be at sustain level
        let mid = buf[buf.len() / 2];
        assert!((mid - 0.3).abs() < 0.01, "Sustain level should be ~0.3, got {mid}");
    }

    #[test]
    fn test_play_note_with_consonance() {
        let preset = builtin_presets()[0].clone();
        let mut synth = ConstraintSynth::with_preset(44100.0, preset);
        synth.consonance_blend = 0.8;
        let buffer = synth.play_note(60, 100, 0.3);
        assert!(!buffer.is_empty());
        for &s in &buffer {
            assert!(s.is_finite(), "Output with consonance must be finite");
        }
    }

    #[test]
    fn test_midi_to_freq_boundaries() {
        // Lowest MIDI note
        let f0 = ConstraintSynth::midi_to_freq(0);
        assert!(f0 > 0.0 && f0 < 20.0, "MIDI 0 should be very low: {f0}");
        // Highest MIDI note
        let f127 = ConstraintSynth::midi_to_freq(127);
        assert!(f127 > 10000.0, "MIDI 127 should be very high: {f127}");
        // C4 = MIDI 60
        let c4 = ConstraintSynth::midi_to_freq(60);
        assert!((c4 - 261.63).abs() < 0.1, "C4 should be ~261.63 Hz: {c4}");
    }

    #[test]
    fn test_play_note_velocity_affects_amplitude() {
        let preset = builtin_presets()[0].clone();
        let mut synth_lo = ConstraintSynth::with_preset(44100.0, preset.clone());
        let mut synth_hi = ConstraintSynth::with_preset(44100.0, preset);
        let lo = synth_lo.play_note(60, 40, 0.5);
        let hi = synth_hi.play_note(60, 120, 0.5);
        let lo_max = lo.iter().fold(0.0f64, |a, &b| a.max(b.abs()));
        let hi_max = hi.iter().fold(0.0f64, |a, &b| a.max(b.abs()));
        assert!(hi_max > lo_max, "Higher velocity should produce louder output: {hi_max} vs {lo_max}");
    }

    #[test]
    fn test_play_note_all_shapes() {
        for shape in [
            LatticeShape::Sine,
            LatticeShape::Square,
            LatticeShape::Saw,
            LatticeShape::Triangle,
            LatticeShape::Eisenstein,
        ] {
            let preset = SynthPreset {
                name: "test".into(),
                shape,
                envelope: AdsrEnvelope::default(),
                filter_type: FilterType::Lowpass,
                filter_cutoff: 2000.0,
                filter_q: 1.0,
                stretch: 1.0,
                noise_floor: 0.0,
            };
            let mut synth = ConstraintSynth::with_preset(44100.0, preset);
            let buf = synth.play_note(64, 100, 0.2);
            assert!(buf.iter().all(|s| s.is_finite()), "Shape {:?} produced non-finite output", shape);
        }
    }

    #[test]
    fn test_synth_preset_serde() {
        let presets = builtin_presets();
        for preset in &presets {
            let json = serde_json::to_string(preset).expect("serialize preset");
            let back: SynthPreset = serde_json::from_str(&json).expect("deserialize preset");
            assert_eq!(back.name, preset.name);
            assert_eq!(back.shape, preset.shape);
            assert!((back.filter_cutoff - preset.filter_cutoff).abs() < 1e-10);
        }
    }

    #[test]
    fn test_builtin_presets_count() {
        let presets = builtin_presets();
        assert_eq!(presets.len(), 5, "Should have 5 built-in presets");
        // Verify all have unique names
        let names: std::collections::HashSet<&str> = presets.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names.len(), 5, "All preset names should be unique");
    }

    #[test]
    fn test_adsr_envelope_all_clamped() {
        let env = AdsrEnvelope {
            attack: 0.1,
            decay: 0.1,
            sustain: 0.5,
            release: 0.1,
        };
        let buf = env.generate(0.3, 1000.0);
        for &v in &buf {
            assert!(v >= 0.0 && v <= 1.0, "Envelope value out of [0,1]: {v}");
        }
    }
}
