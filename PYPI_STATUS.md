# PyPI Publish Status — 2026-05-24

## constraint-audio (Rust/PyO3 wheel)

- **Build**: ✅ `maturin develop --release` compiles and installs successfully
- **Python import**: ✅ `from constraint_audio import PyConstraintSynth` works
- **PyPI publish**: ❌ HTTP 429 "Too many new projects created"
  - PyPI rate-limits new project creation
  - Need to wait (typically 24h, or contact PyPI support for exemption)

### To publish later:
```bash
cd /tmp/constraint-audio-rs
maturin publish --skip-existing
```

## groove-analyzer (pure Python wheel)

- **Dist files ready**: ✅ `groove_analyzer-0.2.0-py3-none-any.whl` + `.tar.gz`
- **PyPI publish**: ❌ HTTP 429 "Too Many Requests"
  - Same rate-limit issue from same account/session

### To publish later:
```bash
cd /tmp/publish/groove-analyzer
python3 -m twine upload dist/* --skip-existing
```

## Recommendation
Wait at least 24 hours and retry both. If urgent, email support@pypi.org requesting a new-project quota increase.
