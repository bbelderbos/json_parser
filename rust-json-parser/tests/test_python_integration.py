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
