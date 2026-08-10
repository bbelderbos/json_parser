from ._rust_json_parser import (
    parse_json,
    parse_json_file,
    dumps,
    benchmark_performance,
    benchmark_serde_json,
)

__all__ = [
    "parse_json",
    "parse_json_file",
    "dumps",
    "benchmark_performance",
    "benchmark_serde_json",
]
