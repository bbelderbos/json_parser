mod error;
mod parser;
mod tokenizer;
mod value;

pub use error::{JsonError, Result};
pub use parser::JsonParser;
pub use tokenizer::Token;
pub use value::JsonValue;

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_json(input: &str) -> Result<JsonValue> {
        let mut parser = JsonParser::new(input)?;
        parser.parse()
    }

    #[test]
    fn test_integration() -> Result<()> {
        // Test the full parsing pipeline
        assert_eq!(parse_json("42")?, JsonValue::Number(42.0));
        assert_eq!(parse_json("true")?, JsonValue::Boolean(true));
        assert_eq!(parse_json("null")?, JsonValue::Null);
        assert_eq!(
            parse_json(r#""hello""#)?,
            JsonValue::String("hello".to_string())
        );
        Ok(())
    }

    #[test]
    fn test_error_propagation() {
        // Test that errors propagate properly with correct details
        let result = parse_json("@invalid@");
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
