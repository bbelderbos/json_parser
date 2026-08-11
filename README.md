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

`--benchmark` times four input shapes against three baselines and prints the ratios.

A parser that stops at a Rust `JsonValue` tree and one that hands back a Python `dict` have
not done the same amount of work, so there are two "ours" columns and each baseline sits
next to the one it's comparable with:

| Input | Ours (Rust tree) | serde_json | Ours (Python objs) | json (C) | simplejson (pure Python) |
|-------|-----------------:|-----------:|-------------------:|---------:|-------------------------:|
| Small (260 B) | 1.65ms | 1.01ms — 1.63x slower | 2.48ms (+1.50x) | 1.47ms — 1.69x slower | 12.91ms — **5.20x faster** |
| Medium (13 KB) | 14.31ms | 9.48ms — 1.51x slower | 22.90ms (+1.60x) | 9.16ms — 2.50x slower | 132.53ms — **5.79x faster** |
| Large (269 KB) | 31.14ms | 19.59ms — 1.59x slower | 47.52ms (+1.53x) | 19.95ms — 2.38x slower | 267.12ms — **5.62x faster** |
| Nested x100 (2.4 KB) | 10.19ms | 6.85ms — 1.49x slower | 14.41ms (+1.41x) | 6.12ms — 2.35x slower | 99.92ms — **6.94x faster** |

*macOS arm64, Python 3.12, release build. Totals for the whole iteration batch (1,000 / 200
/ 20 / 500 respectively), not per parse.*

### What the baselines mean

- **serde_json** — is this parser fast, or is *Rust* fast? This is the only column that
  isolates the quality of the implementation from the choice of language. Being 1.5–1.6x
  behind a heavily tuned reference implementation is the honest measure of where this
  code stands.
- **`json` (C)** — the bar that matters in practice, since it's what a Python developer
  would otherwise import. CPython's `json` is C with 15+ years of tuning behind it.
- **simplejson (pure Python)** — the compiled-versus-interpreted gap, running the same
  algorithm class in the interpreter.

### The price of PyO3

`Ours (Rust tree)` times `parse()`. `Ours (Python objs)` times what `parse_json()` actually
does: `parse()` plus the `IntoPyObject` pass that allocates a `PyDict` per object, a
`PyList` per array, and a Python `float`/`str` per leaf. That conversion is the `+1.4x` to
`+1.6x` in parentheses — **a third to a half of the total cost of the function a Python
caller imports**, on every input shape tested.

Comparing the Rust-tree column against `json.loads` would have been flattering and wrong:
it reads as 1.1–1.6x slower, while the end-to-end truth is 2.4–2.5x slower on anything
bigger than a toy payload. `json.loads` builds those same Python objects and its column
includes that cost, so this is the comparison that holds. The simplejson advantage shrinks
the same way, from ~8x down to ~5.6x.

Note the conversion overhead is roughly flat across sizes, which says it scales with the
*number of values*, not bytes — as expected when the cost is one Python allocation per
node. It's also the most obvious remaining optimization target: `PyDict` preallocation and
interning repeated keys both attack it directly.

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
