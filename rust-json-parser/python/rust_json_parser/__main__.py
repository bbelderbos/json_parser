import sys
from pathlib import Path

from rust_json_parser import parse_json, parse_json_file, dumps


if len(sys.argv) < 2:
    print("Usage: uv run python -m rust_json_parser <json_string_or_file_path>")
    sys.exit(1)

input_arg = sys.argv[1]
path = Path(input_arg)

try:
    if path.is_file():
        result = parse_json_file(str(path))
    else:
        result = parse_json(input_arg)

    print(dumps(result, indent=4))
except FileNotFoundError:
    print(f"Error: File '{input_arg}' not found.")
except ValueError as e:
    print(f"Error: Invalid JSON - {e}")
except Exception as e:
    print(f"An unexpected error occurred: {e}")
