use crate::error::{JsonError, Result};
use crate::tokenizer::{Token, Tokenizer};
use crate::value::JsonValue;
use std::collections::HashMap;

pub struct JsonParser {
    tokens: Vec<Token>,
    position: usize,
}

impl JsonParser {
    pub fn new(input: &str) -> Result<Self> {
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize()?;
        Ok(JsonParser {
            tokens,
            position: 0,
        })
    }

    fn advance(&mut self) -> Option<Token> {
        if self.is_at_end() {
            None
        } else {
            let token = self.tokens[self.position].clone();
            self.position += 1;
            Some(token)
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }

    pub fn parse(&mut self) -> Result<JsonValue> {
        let value = self.parse_value()?;
        let position = self.position;
        match self.advance() {
            None => Ok(value),
            Some(token) => Err(JsonError::UnexpectedToken {
                expected: "end of input".to_string(),
                found: format!("{token:?}"),
                position,
            }),
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue> {
        let position = self.position;
        match self.advance() {
            None => Err(JsonError::UnexpectedEndOfInput {
                expected: "JSON value".to_string(),
                position,
            }),
            Some(Token::Null) => Ok(JsonValue::Null),
            Some(Token::Boolean(b)) => Ok(JsonValue::Boolean(b)),
            Some(Token::Number(n)) => Ok(JsonValue::Number(n)),
            Some(Token::String(s)) => Ok(JsonValue::String(s)),
            Some(Token::LeftBracket) => Ok(self.parse_array()?),
            Some(Token::LeftBrace) => Ok(self.parse_object()?),
            Some(token) => Err(JsonError::UnexpectedToken {
                expected: "null, boolean, number, string, array, or object".to_string(),
                found: format!("{token:?}"),
                position,
            }),
        }
    }

    fn parse_array(&mut self) -> Result<JsonValue> {
        let mut items = Vec::with_capacity(4);
        match self.advance() {
            Some(Token::RightBracket) => return Ok(JsonValue::Array(items)),
            Some(_) => {
                self.position -= 1; // Unconsume the token
                items.push(self.parse_value()?);
            }
            None => {
                return Err(JsonError::UnexpectedEndOfInput {
                    expected: "array value or ']'".to_string(),
                    position: self.position,
                });
            }
        }
        loop {
            match self.advance() {
                Some(Token::RightBracket) => return Ok(JsonValue::Array(items)),
                Some(Token::Comma) => {
                    items.push(self.parse_value()?);
                }
                Some(token) => {
                    return Err(JsonError::UnexpectedToken {
                        expected: "',' or ']'".to_string(),
                        found: format!("{token:?}"),
                        position: self.position - 1,
                    });
                }
                None => {
                    return Err(JsonError::UnexpectedEndOfInput {
                        expected: "',' or ']'".to_string(),
                        position: self.position,
                    });
                }
            }
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue> {
        let mut items = HashMap::with_capacity(8);

        let first_key = match self.advance() {
            Some(Token::RightBrace) => return Ok(JsonValue::Object(items)),
            Some(Token::String(key)) => key,
            Some(token) => {
                return Err(JsonError::UnexpectedToken {
                    expected: "string key or '}'".to_string(),
                    found: format!("{token:?}"),
                    position: self.position - 1,
                });
            }
            None => {
                return Err(JsonError::UnexpectedEndOfInput {
                    expected: "string key".to_string(),
                    position: self.position,
                });
            }
        };
        self.insert_pair(&mut items, first_key)?;

        loop {
            match self.advance() {
                Some(Token::RightBrace) => return Ok(JsonValue::Object(items)),
                Some(Token::Comma) => {
                    let key = match self.advance() {
                        Some(Token::String(key)) => key,
                        Some(token) => {
                            return Err(JsonError::UnexpectedToken {
                                expected: "string key".to_string(),
                                found: format!("{token:?}"),
                                position: self.position - 1,
                            });
                        }
                        None => {
                            return Err(JsonError::UnexpectedEndOfInput {
                                expected: "string key".to_string(),
                                position: self.position,
                            });
                        }
                    };
                    self.insert_pair(&mut items, key)?;
                }
                Some(token) => {
                    return Err(JsonError::UnexpectedToken {
                        expected: "',' or '}'".to_string(),
                        found: format!("{token:?}"),
                        position: self.position - 1,
                    });
                }
                None => {
                    return Err(JsonError::UnexpectedEndOfInput {
                        expected: "',' or '}'".to_string(),
                        position: self.position,
                    });
                }
            }
        }
    }

    fn insert_pair(&mut self, map: &mut HashMap<String, JsonValue>, key: String) -> Result<()> {
        match self.advance() {
            Some(Token::Colon) => {}
            Some(token) => {
                return Err(JsonError::UnexpectedToken {
                    expected: "':'".to_string(),
                    found: format!("{token:?}"),
                    position: self.position - 1,
                });
            }
            None => {
                return Err(JsonError::UnexpectedEndOfInput {
                    expected: "':'".to_string(),
                    position: self.position,
                });
            }
        }
        map.insert(key, self.parse_value()?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn test_parser_creation() {
        let parser = JsonParser::new("42");
        assert!(parser.is_ok());
    }

    #[test]
    fn test_parser_creation_tokenize_error() {
        let parser = JsonParser::new(r#""\q""#); // Invalid escape
        assert!(parser.is_err());
    }

    #[test]
    fn test_parse_number() -> Result<()> {
        for (input, expected) in [("42", 42.0), ("42.5", 42.5), ("0", 0.0)] {
            assert_eq!(parse(input)?, JsonValue::Number(expected));
        }
        Ok(())
    }

    #[test]
    fn test_parse_string() -> Result<()> {
        assert_eq!(parse(r#""hi""#)?, JsonValue::String("hi".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_boolean_true() -> Result<()> {
        assert_eq!(parse("true")?, JsonValue::Boolean(true));
        Ok(())
    }

    #[test]
    fn test_parse_null() -> Result<()> {
        assert_eq!(parse("null")?, JsonValue::Null);
        Ok(())
    }

    #[test]
    fn test_parse_simple_string() -> Result<()> {
        assert_eq!(parse(r#""hello""#)?, JsonValue::String("hello".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_error_trailing_tokens() -> Result<()> {
        for input in ["42 true", "null null", r#""a" "b""#] {
            match parse(input) {
                Err(JsonError::UnexpectedToken { expected, .. }) => {
                    assert_eq!(expected, "end of input");
                }
                _ => panic!("Expected UnexpectedToken error for {input:?}"),
            }
        }
        Ok(())
    }

    #[test]
    fn test_parse_error_empty() -> Result<()> {
        match parse("") {
            Err(JsonError::UnexpectedEndOfInput { expected, position }) => {
                assert_eq!(expected, "JSON value");
                assert_eq!(position, 0);
            }
            _ => panic!("Expected UnexpectedEndOfInput error"),
        }
        Ok(())
    }

    #[test]
    fn test_parse_negative_number() -> Result<()> {
        assert_eq!(parse("-3.25")?, JsonValue::Number(-3.25));
        Ok(())
    }

    #[test]
    fn test_parse_boolean_false() -> Result<()> {
        assert_eq!(parse("false")?, JsonValue::Boolean(false));
        Ok(())
    }

    #[test]
    fn test_parse_whitespace_only() {
        assert!(parse("   ").is_err());
    }

    #[test]
    fn test_parse_string_with_newline() -> Result<()> {
        let value = parse(r#""hello\nworld""#)?;
        assert_eq!(value, JsonValue::String("hello\nworld".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_string_with_unicode() -> Result<()> {
        let value = parse(r#""\u0048\u0065\u006c\u006c\u006f""#)?;
        assert_eq!(value, JsonValue::String("Hello".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_complex_escapes() -> Result<()> {
        let value = parse(r#""line1\nline2\t\"quoted\"\u0021""#)?;
        assert_eq!(
            value,
            JsonValue::String("line1\nline2\t\"quoted\"!".to_string())
        );
        Ok(())
    }

    #[test]
    fn test_parse_string_with_tab() -> Result<()> {
        let value = parse(r#""col1\tcol2""#)?;
        assert_eq!(value, JsonValue::String("col1\tcol2".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_string_with_quotes() -> Result<()> {
        let value = parse(r#""say \"hi\"""#)?;
        assert_eq!(value, JsonValue::String("say \"hi\"".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_error_invalid_token() {
        assert!(JsonParser::new("@").is_err());
    }

    #[test]
    fn test_parse_with_whitespace() -> Result<()> {
        assert_eq!(parse("  42  ")?, JsonValue::Number(42.0));

        assert_eq!(parse("\n\ttrue\n")?, JsonValue::Boolean(true));
        Ok(())
    }

    #[test]
    fn test_parse_empty_array() -> Result<()> {
        let mut parser = JsonParser::new("[]")?;
        let value = parser.parse()?;
        assert_eq!(value, JsonValue::Array(vec![]));
        Ok(())
    }

    #[test]
    fn test_parse_array_single() -> Result<()> {
        let mut parser = JsonParser::new("[1]")?;
        let value = parser.parse()?;
        assert_eq!(value, JsonValue::Array(vec![JsonValue::Number(1.0)]));
        Ok(())
    }

    #[test]
    fn test_parse_array_multiple() -> Result<()> {
        let mut parser = JsonParser::new("[1, 2, 3]")?;
        let value = parser.parse()?;
        let expected = JsonValue::Array(vec![
            JsonValue::Number(1.0),
            JsonValue::Number(2.0),
            JsonValue::Number(3.0),
        ]);
        assert_eq!(value, expected);
        Ok(())
    }

    #[test]
    fn test_parse_array_mixed_types() -> Result<()> {
        let mut parser = JsonParser::new(r#"[1, "two", true, null]"#)?;
        let value = parser.parse()?;
        let expected = JsonValue::Array(vec![
            JsonValue::Number(1.0),
            JsonValue::String("two".to_string()),
            JsonValue::Boolean(true),
            JsonValue::Null,
        ]);
        assert_eq!(value, expected);
        Ok(())
    }

    #[test]
    fn test_array_accessor() -> Result<()> {
        let mut parser = JsonParser::new("[1, 2, 3]")?;
        let value = parser.parse()?;
        assert_eq!(value.as_array().map(Vec::len), Some(3));
        Ok(())
    }

    #[test]
    fn test_array_get_index() -> Result<()> {
        let mut parser = JsonParser::new("[10, 20, 30]")?;
        let value = parser.parse()?;
        assert_eq!(value.get_index(1), Some(&JsonValue::Number(20.0)));
        assert_eq!(value.get_index(5), None);
        Ok(())
    }

    #[test]
    fn test_parse_empty_object() -> Result<()> {
        let mut parser = JsonParser::new("{}")?;
        let value = parser.parse()?;
        assert_eq!(value, JsonValue::Object(HashMap::new()));
        Ok(())
    }

    #[test]
    fn test_parse_object_single_key() -> Result<()> {
        let mut parser = JsonParser::new(r#"{"key": "value"}"#)?;
        let value = parser.parse()?;
        let mut expected = HashMap::new();
        expected.insert("key".to_string(), JsonValue::String("value".to_string()));
        assert_eq!(value, JsonValue::Object(expected));
        Ok(())
    }

    #[test]
    fn test_parse_object_multiple_keys() -> Result<()> {
        let mut parser = JsonParser::new(r#"{"name": "Alice", "age": 30}"#)?;
        let value = parser.parse()?;
        if let JsonValue::Object(obj) = value {
            assert_eq!(
                obj.get("name"),
                Some(&JsonValue::String("Alice".to_string()))
            );
            assert_eq!(obj.get("age"), Some(&JsonValue::Number(30.0)));
        } else {
            panic!("Expected object");
        }
        Ok(())
    }

    #[test]
    fn test_object_accessor() -> Result<()> {
        let mut parser = JsonParser::new(r#"{"name": "test"}"#)?;
        let value = parser.parse()?;
        assert_eq!(value.as_object().map(HashMap::len), Some(1));
        Ok(())
    }

    #[test]
    fn test_object_get() -> Result<()> {
        let mut parser = JsonParser::new(r#"{"name": "Alice", "age": 30}"#)?;
        let value = parser.parse()?;
        assert_eq!(
            value.get("name"),
            Some(&JsonValue::String("Alice".to_string()))
        );
        assert_eq!(value.get("missing"), None);
        Ok(())
    }

    #[test]
    fn test_parse_nested_arrays() -> Result<()> {
        let mut parser = JsonParser::new("[[1, 2], [3, 4]]")?;
        let value = parser.parse()?;
        let expected = JsonValue::Array(vec![
            JsonValue::Array(vec![JsonValue::Number(1.0), JsonValue::Number(2.0)]),
            JsonValue::Array(vec![JsonValue::Number(3.0), JsonValue::Number(4.0)]),
        ]);
        assert_eq!(value, expected);
        Ok(())
    }

    #[test]
    fn test_parse_deeply_nested() -> Result<()> {
        let mut parser = JsonParser::new("[[[1]]]")?;
        let value = parser.parse()?;
        let expected = JsonValue::Array(vec![JsonValue::Array(vec![JsonValue::Array(vec![
            JsonValue::Number(1.0),
        ])])]);
        assert_eq!(value, expected);
        Ok(())
    }

    #[test]
    fn test_parse_nested_object() -> Result<()> {
        let mut parser = JsonParser::new(r#"{"outer": {"inner": 1}}"#)?;
        let value = parser.parse()?;
        if let JsonValue::Object(outer) = value {
            if let Some(JsonValue::Object(inner)) = outer.get("outer") {
                assert_eq!(inner.get("inner"), Some(&JsonValue::Number(1.0)));
            } else {
                panic!("Expected nested object");
            }
        } else {
            panic!("Expected object");
        }
        Ok(())
    }

    #[test]
    fn test_parse_array_in_object() -> Result<()> {
        let mut parser = JsonParser::new(r#"{"items": [1, 2, 3]}"#)?;
        let value = parser.parse()?;
        if let JsonValue::Object(obj) = value {
            if let Some(JsonValue::Array(arr)) = obj.get("items") {
                assert_eq!(arr.len(), 3);
            } else {
                panic!("Expected array");
            }
        } else {
            panic!("Expected object");
        }
        Ok(())
    }

    #[test]
    fn test_parse_object_in_array() -> Result<()> {
        let mut parser = JsonParser::new(r#"[{"a": 1}, {"b": 2}]"#)?;
        let value = parser.parse()?;
        if let JsonValue::Array(arr) = value {
            assert_eq!(arr.len(), 2);
        } else {
            panic!("Expected array");
        }
        Ok(())
    }

    #[test]
    fn test_error_unclosed_array() -> Result<()> {
        let mut parser = JsonParser::new("[1, 2")?;
        let result = parser.parse();
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_error_unclosed_object() -> Result<()> {
        let mut parser = JsonParser::new(r#"{"key": 1"#)?;
        let result = parser.parse();
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_error_trailing_comma_array() -> Result<()> {
        let mut parser = JsonParser::new("[1, 2,]")?;
        let result = parser.parse();
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_error_trailing_comma_object() -> Result<()> {
        let mut parser = JsonParser::new(r#"{"a": 1,}"#)?;
        let result = parser.parse();
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_error_missing_colon() -> Result<()> {
        let mut parser = JsonParser::new(r#"{"key" 1}"#)?;
        let result = parser.parse();
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_error_invalid_key() -> Result<()> {
        let mut parser = JsonParser::new(r#"{123: "value"}"#)?;
        let result = parser.parse();
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_error_missing_comma_array() -> Result<()> {
        let mut parser = JsonParser::new("[1 2 3]")?;
        let result = parser.parse();
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_error_missing_comma_object() -> Result<()> {
        let mut parser = JsonParser::new(r#"{"a": 1 "b": 2}"#)?;
        let result = parser.parse();
        assert!(result.is_err());
        Ok(())
    }
}
