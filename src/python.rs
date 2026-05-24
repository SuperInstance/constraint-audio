//! PyO3 Python bindings for constraint-audio.

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

use crate::lattice_oscillator::{LatticeOscillator, LatticeShape};
use crate::synth::{builtin_presets, ConstraintSynth};

/// Parse a shape name string into a LatticeShape enum.
fn parse_shape(name: &str) -> PyResult<LatticeShape> {
    match name.to_lowercase().as_str() {
        "sine" => Ok(LatticeShape::Sine),
        "square" => Ok(LatticeShape::Square),
        "saw" | "sawtooth" => Ok(LatticeShape::Saw),
        "triangle" | "tri" => Ok(LatticeShape::Triangle),
        "eisenstein" | "eis" => Ok(LatticeShape::Eisenstein),
        other => Err(PyValueError::new_err(format!(
            "Unknown shape '{other}'. Use: sine, square, saw, triangle, eisenstein"
        ))),
    }
}

/// Convert a Vec<f64> into a numpy array (f64).
fn vec_to_numpy<'py>(py: Python<'py>, data: Vec<f64>) -> PyResult<Bound<'py, PyAny>> {
    let np = py.import_bound("numpy")?;
    let arr = np.call_method1("array", (data,))?;
    arr.call_method1("astype", ("float64",))
}

// ── PyLatticeOscillator ──────────────────────────────────────────────

#[pyclass(name = "PyLatticeOscillator")]
struct PyLatticeOscillator {
    inner: LatticeOscillator,
}

#[pymethods]
impl PyLatticeOscillator {
    #[new]
    #[pyo3(signature = (frequency=440.0, sample_rate=44100.0, shape_name="sine".into()))]
    fn new(frequency: f64, sample_rate: f64, shape_name: &str) -> PyResult<Self> {
        let shape = parse_shape(shape_name)?;
        Ok(Self {
            inner: LatticeOscillator::new(frequency, sample_rate, shape),
        })
    }

    /// Generate audio for `duration_secs` and return a numpy float64 array.
    fn generate<'py>(&mut self, py: Python<'py>, duration_secs: f64) -> PyResult<Bound<'py, PyAny>> {
        let data = self.inner.generate(duration_secs);
        vec_to_numpy(py, data)
    }

    /// Reset the oscillator phase to zero.
    fn reset(&mut self) {
        self.inner.reset();
    }

    #[getter]
    fn frequency(&self) -> f64 {
        self.inner.freq
    }

    #[getter]
    fn sample_rate(&self) -> f64 {
        self.inner.sample_rate
    }
}

// ── PyConstraintSynth ────────────────────────────────────────────────

#[pyclass(name = "PyConstraintSynth")]
struct PyConstraintSynth {
    inner: ConstraintSynth,
}

#[pymethods]
impl PyConstraintSynth {
    #[new]
    #[pyo3(signature = (sample_rate=44100.0))]
    fn new(sample_rate: f64) -> Self {
        Self {
            inner: ConstraintSynth::new(sample_rate),
        }
    }

    #[staticmethod]
    #[pyo3(signature = (preset_name, sample_rate=44100.0))]
    fn from_preset(preset_name: &str, sample_rate: f64) -> PyResult<Self> {
        let name_lower = preset_name.to_lowercase();
        let preset = builtin_presets()
            .into_iter()
            .find(|p| p.name.to_lowercase() == name_lower)
            .ok_or_else(|| {
                let names: Vec<String> =
                    builtin_presets().iter().map(|p| p.name.clone()).collect();
                PyValueError::new_err(format!(
                    "Unknown preset '{preset_name}'. Available: {}",
                    names.join(", ")
                ))
            })?;
        Ok(Self {
            inner: ConstraintSynth::with_preset(sample_rate, preset),
        })
    }

    /// Play a single note. Returns numpy float64 array.
    fn play_note<'py>(
        &mut self,
        py: Python<'py>,
        pitch: u8,
        velocity: u8,
        duration: f64,
    ) -> PyResult<Bound<'py, PyAny>> {
        let data = self.inner.play_note(pitch, velocity, duration);
        vec_to_numpy(py, data)
    }

    /// Render a melody from a list of (pitch, velocity, duration) tuples.
    /// Returns numpy float64 array with all notes concatenated.
    fn render_melody<'py>(
        &mut self,
        py: Python<'py>,
        notes: Vec<(u8, u8, f64)>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut combined: Vec<f64> = Vec::new();
        for (pitch, velocity, duration) in notes {
            let buf = self.inner.play_note(pitch, velocity, duration);
            combined.extend_from_slice(&buf);
        }
        vec_to_numpy(py, combined)
    }

    /// List available preset names.
    #[staticmethod]
    fn list_presets() -> Vec<String> {
        builtin_presets().iter().map(|p| p.name.clone()).collect()
    }

    #[getter]
    fn sample_rate(&self) -> f64 {
        self.inner.sample_rate
    }
}

// ── Module ───────────────────────────────────────────────────────────

#[pymodule]
fn constraint_audio(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyLatticeOscillator>()?;
    m.add_class::<PyConstraintSynth>()?;
    Ok(())
}
