import importlib.util
import json
import sys
from pathlib import Path

from rich.console import Console
from rich.table import Table

from rust_json_parser import (
    parse_json,
    parse_json_file,
    dumps,
    benchmark_performance,
    benchmark_parse_json,
    benchmark_serde_json,
)


NESTING_DEPTH = 100


def is_file(value: str) -> bool:
    try:
        return Path(value).is_file()
    except OSError:
        return False


def require_pure_python_simplejson() -> None:
    """A C-accelerated simplejson turns its column into a second Rust-vs-C comparison."""
    if importlib.util.find_spec("simplejson._speedups") is None:
        return
    sys.exit(
        "simplejson has its C extension installed, so its timings would be "
        "meaningless here. Reinstall the pure-Python build with:\n"
        '  DISABLE_SPEEDUPS=1 uv pip install "simplejson>=3.19.0" --no-binary simplejson'
    )


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


def load_fixture(name: str) -> str:
    path = Path(__file__).parents[2] / "benches" / "data" / f"{name}.json"
    return path.read_text(encoding="utf-8")


def benchmark_inputs() -> list[tuple[str, str, int, int]]:
    return [
        ("Small", make_test_json(1), 1000, 100),
        ("Medium", make_test_json(50), 200, 50),
        (f"Nested x{NESTING_DEPTH}", make_nested_json(NESTING_DEPTH), 500, 100),
        ("Twitter\n(strings)", load_fixture("twitter"), 100, 20),
        ("Citm\n(mixed)", load_fixture("citm_catalog"), 50, 10),
        ("Canada\n(floats)", load_fixture("canada"), 50, 10),
    ]


def ms(seconds: float) -> str:
    return f"{seconds * 1000:.2f}ms"


def speedup(ours: float, other: float) -> str:
    ratio = other / ours
    if ratio >= 1:
        return f"{ms(other)}\n[green]{ratio:.2f}x faster[/green]"
    return f"{ms(other)}\n[red]{1 / ratio:.2f}x slower[/red]"


def run_benchmarks() -> None:
    require_pure_python_simplejson()

    table = Table(
        title="JSON parsing: our Rust parser vs the alternatives",
        caption="Each baseline is compared against the column that does the same work.\n"
        "Release build; simplejson C speedups disabled.",
    )
    table.add_column("Input")
    table.add_column("Ours\n(Rust tree)", justify="right")
    table.add_column("serde_json")
    table.add_column("Ours\n(Python objs)", justify="right")
    table.add_column("json (C)")
    table.add_column("simplejson\n(pure py)")

    for label, test_json, iterations, warmup in benchmark_inputs():
        rust, py_json, simple = benchmark_performance(test_json, iterations, warmup)
        serde = benchmark_serde_json(test_json, iterations, warmup)
        converted = benchmark_parse_json(test_json, iterations, warmup)
        table.add_row(
            f"{label}\n[dim]{len(test_json):,} B x{iterations:,}[/dim]",
            ms(rust),
            speedup(rust, serde),
            f"{ms(converted)}\n[dim]+{converted / rust:.2f}x[/dim]",
            speedup(converted, py_json),
            speedup(converted, simple),
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
    print("Usage: uv run python -m rust_json_parser <json_string_or_file_path>", file=sys.stderr)
    print("       ... | uv run python -m rust_json_parser", file=sys.stderr)
    sys.exit(1)

try:
    print(dumps(parse(input_arg), indent=4))
except OSError as e:
    print(f"Error: cannot read '{input_arg}' - {e}", file=sys.stderr)
    sys.exit(1)
except ValueError as e:
    print(f"Error: Invalid JSON - {e}", file=sys.stderr)
    sys.exit(1)
except Exception as e:
    print(f"An unexpected error occurred: {e}", file=sys.stderr)
    sys.exit(1)
