import json
import sys
from pathlib import Path

import simplejson
import simplejson.decoder
import simplejson.scanner
from rich.console import Console
from rich.table import Table

from rust_json_parser import (
    parse_json,
    parse_json_file,
    dumps,
    benchmark_performance,
    benchmark_serde_json,
)


NESTING_DEPTH = 100


def is_file(value: str) -> bool:
    try:
        return Path(value).is_file()
    except OSError:
        return False


def use_pure_python_simplejson() -> None:
    """simplejson's wheel bundles a C decoder, so by default it measures Rust vs C."""
    simplejson.decoder.scanstring = simplejson.decoder.py_scanstring
    simplejson.decoder.make_scanner = simplejson.scanner.py_make_scanner
    simplejson._default_decoder = simplejson.decoder.JSONDecoder()


def make_record(i: int) -> dict:
    return {
        "id": i,
        "name": f"user_{i}",
        "email": f"user_{i}@example.com",
        "bio": 'quotes " backslash \\ tab \t café',
        "score": (i % 90) + 0.5,
        "balance": -1.25e3,
        "active": i % 2 == 0,
        "manager": None,
        "tags": ["alpha", "beta", "gamma"],
        "address": {"city": f"City_{i}", "zip": f"{10000 + i}"},
    }


def make_test_json(records: int) -> str:
    return json.dumps([make_record(i) for i in range(records)])


def make_nested_json(depth: int) -> str:
    opening = "".join(f'{{"level": {i}, "child": ' for i in range(depth))
    return f"{opening}null{'}' * depth}"


def benchmark_inputs() -> list[tuple[str, str, int]]:
    return [
        ("Small", make_test_json(1), 1000),
        ("Medium", make_test_json(50), 200),
        ("Large", make_test_json(1000), 20),
        (f"Nested x{NESTING_DEPTH}", make_nested_json(NESTING_DEPTH), 500),
    ]


def speedup(rust: float, other: float) -> str:
    ratio = other / rust
    if ratio >= 1:
        return f"[green]{ratio:.2f}x faster[/green]"
    return f"[red]{1 / ratio:.2f}x slower[/red]"


def run_benchmarks() -> None:
    use_pure_python_simplejson()

    table = Table(
        title="JSON parsing: our Rust parser vs the alternatives",
        caption="Ratios are our parser against each baseline. Release build; simplejson C speedups disabled.",
    )
    table.add_column("Input")
    table.add_column("Bytes", justify="right")
    table.add_column("Iters", justify="right")
    table.add_column("Ours", justify="right")
    table.add_column("serde_json")
    table.add_column("json (C)")
    table.add_column("simplejson (py)")

    for label, test_json, iterations in benchmark_inputs():
        rust, py_json, simple = benchmark_performance(test_json, iterations)
        serde = benchmark_serde_json(test_json, iterations)
        table.add_row(
            label,
            f"{len(test_json):,}",
            f"{iterations:,}",
            f"{rust:.6f}s",
            f"{serde:.6f}s  {speedup(rust, serde)}",
            f"{py_json:.6f}s  {speedup(rust, py_json)}",
            f"{simple:.6f}s  {speedup(rust, simple)}",
        )

    Console().print(table)


if len(sys.argv) > 1:
    input_arg = sys.argv[1]
    if input_arg == "--benchmark":
        run_benchmarks()
        sys.exit(0)
    parse = parse_json_file if is_file(input_arg) else parse_json
elif not sys.stdin.isatty():
    input_arg = sys.stdin.read()
    parse = parse_json
else:
    print("Usage: uv run python -m rust_json_parser <json_string_or_file_path>")
    print("       ... | uv run python -m rust_json_parser")
    sys.exit(1)

try:
    print(dumps(parse(input_arg), indent=4))
except FileNotFoundError:
    print(f"Error: File '{input_arg}' not found.")
    sys.exit(1)
except ValueError as e:
    print(f"Error: Invalid JSON - {e}")
    sys.exit(1)
except Exception as e:
    print(f"An unexpected error occurred: {e}")
    sys.exit(1)
