use crate::error::{JsonError, Result};
use crate::tokenizer::{Token, Tokenizer};
use crate::value::JsonValue;

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
        match self.advance() {
            None => Ok(value),
            Some(token) => Err(JsonError::UnexpectedToken {
                expected: "end of input".to_string(),
                found: format!("{token:?}"),
                position: self.position,
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
            Some(token) => Err(JsonError::UnexpectedToken {
                expected: "boolean, number, string or null".to_string(),
                found: format!("{token:?}"),
                position,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            let mut parser = JsonParser::new(input)?;
            assert_eq!(parser.parse()?, JsonValue::Number(expected));
        }
        Ok(())
    }

    #[test]
    fn test_parse_string() -> Result<()> {
        let mut parser = JsonParser::new(r#""hi""#)?;
        assert_eq!(parser.parse()?, JsonValue::String("hi".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_boolean_true() -> Result<()> {
        let mut parser = JsonParser::new("true")?;
        let value = parser.parse()?;
        assert_eq!(value, JsonValue::Boolean(true));
        Ok(())
    }

    #[test]
    fn test_parse_null() -> Result<()> {
        let mut parser = JsonParser::new("null")?;
        let value = parser.parse()?;
        assert_eq!(value, JsonValue::Null);
        Ok(())
    }

    #[test]
    fn test_parse_simple_string() -> Result<()> {
        let mut parser = JsonParser::new(r#""hello""#)?;
        let value = parser.parse()?;
        assert_eq!(value, JsonValue::String("hello".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_error_trailing_tokens() -> Result<()> {
        for input in ["42 true", "null null", r#""a" "b""#] {
            let mut parser = JsonParser::new(input)?;
            match parser.parse() {
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
        let mut parser = JsonParser::new("")?;
        match parser.parse() {
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
        let mut parser = JsonParser::new("-3.14")?;
        let value = parser.parse()?;
        assert_eq!(value, JsonValue::Number(-3.14));
        Ok(())
    }

    #[test]
    fn test_parse_boolean_false() -> Result<()> {
        let mut parser = JsonParser::new("false")?;
        let value = parser.parse()?;
        assert_eq!(value, JsonValue::Boolean(false));
        Ok(())
    }

    #[test]
    fn test_parse_empty_input() {
        // Could fail at tokenization (no tokens) or parsing (empty token list)
        // Either is acceptable - just verify it's an error
        let result = match JsonParser::new("") {
            Ok(mut parser) => parser.parse(),
            Err(e) => Err(e),
        };
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let result = match JsonParser::new("   ") {
            Ok(mut parser) => parser.parse(),
            Err(e) => Err(e),
        };
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_string_with_newline() -> Result<()> {
        let mut parser = JsonParser::new(r#""hello\nworld""#)?;
        let value = parser.parse()?;
        assert_eq!(value, JsonValue::String("hello\nworld".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_string_with_unicode() -> Result<()> {
        let mut parser = JsonParser::new(r#""\u0048\u0065\u006c\u006c\u006f""#)?;
        let value = parser.parse()?;
        assert_eq!(value, JsonValue::String("Hello".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_complex_escapes() -> Result<()> {
        let mut parser = JsonParser::new(r#""line1\nline2\t\"quoted\"\u0021""#)?;
        let value = parser.parse()?;
        assert_eq!(
            value,
            JsonValue::String("line1\nline2\t\"quoted\"!".to_string())
        );
        Ok(())
    }

    #[test]
    fn test_parse_string_with_tab() -> Result<()> {
        let mut parser = JsonParser::new(r#""col1\tcol2""#)?;
        let value = parser.parse()?;
        assert_eq!(value, JsonValue::String("col1\tcol2".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_string_with_quotes() -> Result<()> {
        let mut parser = JsonParser::new(r#""say \"hi\"""#)?;
        let value = parser.parse()?;
        assert_eq!(value, JsonValue::String("say \"hi\"".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_error_invalid_token() {
        assert!(JsonParser::new("@").is_err());
    }

    #[test]
    fn test_parser_new_with_invalid_input() {}

    #[test]
    fn test_parse_with_whitespace() -> Result<()> {
        let mut parser = JsonParser::new("  42  ")?;
        assert_eq!(parser.parse()?, JsonValue::Number(42.0));

        let mut parser = JsonParser::new("\n\ttrue\n")?;
        assert_eq!(parser.parse()?, JsonValue::Boolean(true));
        Ok(())
    }
}
