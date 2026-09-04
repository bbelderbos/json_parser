import sys
from pathlib import Path

from rust_json_parser import parse_json, parse_json_file, dumps


def is_file(value: str) -> bool:
    try:
        return Path(value).is_file()
    except OSError:
        return False


if len(sys.argv) > 1:
    input_arg = sys.argv[1]
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
