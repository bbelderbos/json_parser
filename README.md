# rust-json-parser

A JSON parser written from scratch in Rust, with Python bindings via PyO3. The parser itself
uses no `serde` and no external parsing crates — a hand-written tokenizer and recursive-descent
parser, exposed to Python as a native extension module. (`serde_json` is pulled in only as a
benchmark baseline to compare against, never for parsing.)

Built over six weeks as the capstone of the [Python to Rust](https://scriptertorust.com) cohort.

## Install

The Python package and Rust crate live in `rust-json-parser/`; run everything from there.

```bash
cd rust-json-parser
uv sync
uv run maturin develop --release
```

The `--release` flag is not optional if you care about speed: a debug build is roughly 10x
slower and will make every benchmark number meaningless.

## Usage

### Command line

```bash
# Parse a file
uv run python -m rust_json_parser data.json

# Parse a string
uv run python -m rust_json_parser '{"name": "Alice", "age": 30}'

# Read from stdin
cat data.json | uv run python -m rust_json_parser

# Compare against the alternatives
uv run python -m rust_json_parser --benchmark
```

### Python API

```python
from rust_json_parser import parse_json, parse_json_file, dumps

parse_json('{"name": "Alice"}')      # {'name': 'Alice'}
parse_json('[1, true, null]')        # [1.0, True, None]
parse_json_file("config.json")       # {'debug': True}

dumps({"a": [1, 2]})                 # '{"a":[1,2]}'
dumps({"a": [1, 2]}, indent=4)       # pretty-printed
```

Malformed input raises `ValueError` with the position where parsing failed. Missing files
raise `FileNotFoundError`, unreadable ones `PermissionError`.

## Benchmarking

`--benchmark` times four input shapes against three baselines and prints the ratios:

| Input | Bytes | Ours | serde_json | json (C) | simplejson (pure Python) |
|-------|------:|-----:|-----------:|---------:|-------------------------:|
| Small | 260 | 0.001627s | 1.57x slower | 1.09x slower | **8.05x faster** |
| Medium | 13,225 | 0.015093s | 1.62x slower | 1.61x slower | **8.46x faster** |
| Large | 268,940 | 0.031652s | 1.38x slower | 1.55x slower | **8.21x faster** |
| Nested x100 | 2,394 | 0.010068s | 1.40x slower | 1.61x slower | **8.80x faster** |

*macOS arm64, Python 3.12, release build. Ratios are our parser against each baseline.*

### What the baselines mean

Three references, because each answers a different question:

- **serde_json** — is this parser fast, or is *Rust* fast? This is the only column that
  isolates the quality of the implementation from the choice of language. Being 1.4–1.6x
  behind a heavily tuned reference implementation is the honest measure of where this
  code stands.
- **`json` (C)** — the bar that matters in practice, since it's what a Python developer
  would otherwise import. CPython's `json` is C with 15+ years of tuning behind it.
- **simplejson (pure Python)** — the compiled-versus-interpreted gap, running the same
  algorithm class in the interpreter.

### The simplejson trap

simplejson ships an optional `_speedups` C extension, and the PyPI wheels have it compiled.
Left alone, `simplejson.loads()` runs at C speed and the third column silently becomes a
second Rust-vs-C comparison — showing roughly 1x instead of the expected 8x.

Check your install:

```python
>>> import simplejson
>>> simplejson._speedups   # ImportError means you're fine
```

Fix it by installing the pure-Python build:

```bash
DISABLE_SPEEDUPS=1 uv pip install "simplejson>=3.19.0" --no-binary simplejson
```

`pyproject.toml` pins `no-binary-package = ["simplejson"]` so `uv sync` won't pull the wheel
back, but note that `--no-binary` alone only forces a *source* build — simplejson still
compiles `_speedups` unless `DISABLE_SPEEDUPS=1` is set. Because that env var lives nowhere
in the project files, `run_benchmarks()` checks at runtime and exits with instructions rather
than reporting numbers that look fine and aren't.

### Methodology

- **Release builds only.** The single biggest factor, worth ~10x.
- **Warmup.** Each parser runs 100 untimed parses before measurement so cold caches and
  allocator startup stay out of the numbers.
- **Iteration counts** scale down with payload size (1,000 for Small, 20 for Large). The
  low counts on big inputs are the weak point: repeat runs of the Large row spread ~15%,
  versus ~5% at 200 iterations. Treat single-run differences under ~15% there as noise.
- **Repeat and take the minimum** when comparing builds. The minimum discards scheduler
  interference in a way the mean does not.

Payloads are generated in `__main__.py` rather than read from fixture files, which keeps the
repo small but means numbers are only comparable across machines running the same generator.
For cross-machine comparison, use fixed fixtures — ideally the standard corpus
(`twitter.json`, `citm_catalog.json`, `canada.json`).

## Development

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test                  # 125 unit tests + doc tests
uv run pytest -q            # 12 Python integration tests
```

`cargo test --doc --no-default-features` runs the Rust doc examples without linking Python.
