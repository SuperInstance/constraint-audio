# constraint-audio

Rust audio DSP built on lattice consonance theory — harmonic oscillators, consonance filters, and ADSR synthesis with Python bindings.

## What This Gives You

- **Lattice oscillators** — generate tones at Eisenstein lattice points (2^a × 3^b × 5^c)
- **Consonance scoring** — rate harmonic combinations by lattice distance
- **Consonance filters** — remove dissonant partials from audio streams
- **ADSR synthesis** — full Attack-Decay-Sustain-Release envelope shaping
- **Python bindings** — use from Python via PyO3

## Quick Start

### Rust

```rust
use constraint_audio::{LatticeOscillator, ConsonanceFilter, AdsrEnvelope};

// Create a lattice-based oscillator at A3 (110 Hz)
let mut osc = LatticeOscillator::new(110.0, 44100);

// Apply consonance filter — only keep partials with score > 0.7
let mut filter = ConsonanceFilter::new(0.7);

// Shape with ADSR
let adsr = AdsrEnvelope::new(0.01, 0.1, 0.6, 0.3);

// Render 1 second of audio
let samples: Vec<f32> = (0..44100)
    .map(|i| {
        let t = i as f32 / 44100.0;
        let env = adsr.sample(t);
        let sig = osc.sample(t);
        filter.process(sig * env)
    })
    .collect();
```

### Python

```python
import constraint_audio

# Lattice consonance scoring
score = constraint_audio.consonance_score(frequency=440.0, lattice=(1, 0, 0))
print(f"Consonance: {score:.3f}")

# Generate consonant tones
osc = constraint_audio.LatticeOscillator(110.0, 44100)
audio = osc.render(duration=1.0)
```

## API Reference

| Type | Description |
|---|---|
| `LatticeOscillator` | Generates tones at harmonic lattice points |
| `ConsonanceFilter` | Filters partials below consonance threshold |
| `ConsonanceHeatmap` | Tracks consonance over frequency × time |
| `AdsrdEnvelope` | ADSR envelope generator |
| `consonance_score(freq, lattice)` | Rate a frequency/lattice combination |

## How It Fits

The **audio DSP layer** of the constraint theory ecosystem:

- [constraint-theory-core](https://github.com/SuperInstance/constraint-theory-core) — lattice theory behind the oscillators
- [constraint-synth](https://github.com/SuperInstance/constraint-synth) — full synthesizer built on this library
- [constraint-mux](https://github.com/SuperInstance/constraint-mux) — serial multiplexer with real-time consonance analysis
- [constraint-instrument](https://github.com/SuperInstance/constraint-instrument) — musician-facing instrument API

## Testing

```bash
cargo test
```

46 tests covering oscillators, filters, consonance scoring, and ADSR envelopes.

## Installation

```bash
# Rust
cargo add constraint-audio

# Python
pip install constraint-audio
```

## License

MIT

## Documentation

📚 [OpenConstruct Docs](https://github.com/SuperInstance/openconstruct-docs)
