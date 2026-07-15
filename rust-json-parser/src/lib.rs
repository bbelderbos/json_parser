mod error;
mod parser;
mod tokenizer;
mod value;

pub use error::{JsonError, Result};
pub use parser::JsonParser;
pub use tokenizer::Token;
pub use value::JsonValue;

/// Parse a JSON document into a [`JsonValue`].
pub fn parse(input: &str) -> Result<JsonValue> {
    JsonParser::new(input)?.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration() -> Result<()> {
        // Test the full parsing pipeline
        assert_eq!(parse("42")?, JsonValue::Number(42.0));
        assert_eq!(parse("true")?, JsonValue::Boolean(true));
        assert_eq!(parse("null")?, JsonValue::Null);
        assert_eq!(parse(r#""hello""#)?, JsonValue::String("hello".to_string()));
        Ok(())
    }

    #[test]
    fn test_error_propagation() {
        // Test that errors propagate properly with correct details
        let result = parse("@invalid@");
        assert!(result.is_err());

        // Validate error details through pattern matching
        match result {
            Err(JsonError::UnexpectedToken {
                expected,
                found,
                position,
            }) => {
                assert_eq!(expected, "valid JSON token");
                assert_eq!(found, "@");
                assert_eq!(position, 0);
            }
            _ => panic!("Expected UnexpectedToken error"),
        }
    }
}
