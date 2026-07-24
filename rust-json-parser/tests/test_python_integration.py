import pytest

from rust_json_parser import parse_json


class TestTypeConversions:
    def test_null_becomes_none(self):
        result = parse_json('{"value": null}')
        assert result["value"] is None

    def test_bool_stays_bool(self):
        result = parse_json('{"t": true, "f": false}')
        assert result["t"] is True
        assert result["f"] is False
        assert isinstance(result["t"], bool)

    def test_numbers_are_float(self):
        result = parse_json('{"int": 42, "float": 3.14}')
        assert result["int"] == 42.0
        assert result["float"] == 3.14


class TestErrorHandling:
    def test_parse_error_raises_value_error(self):
        with pytest.raises(ValueError):
            parse_json('{"unclosed": "string')

    def test_error_includes_position(self):
        try:
            parse_json('{"bad": }')
        except ValueError as e:
            assert "position" in str(e).lower()
