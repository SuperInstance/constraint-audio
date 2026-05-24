//! # constraint-audio
//!
//! Audio DSP backend for the constraint-theory ecosystem.
//!
//! Provides lattice-based oscillators, constraint-theory filters, and a
//! synthesizer for offline audio buffer rendering.

pub mod constraint_filter;
pub mod lattice_oscillator;
pub mod synth;

// Re-export main types for convenience
pub use constraint_filter::{BiquadFilter, ConsonanceFilter, FilterType};
pub use lattice_oscillator::{LatticeOscillator, LatticeShape};
pub use synth::{builtin_presets, AdsrEnvelope, ConstraintSynth, SynthPreset};
