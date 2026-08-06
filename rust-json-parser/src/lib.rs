//! A hand-written JSON parser: text in, [`JsonValue`] tree out.
//!
//! Parsing runs in two stages, a [`Token`] scanner followed by a recursive-descent
//! [`JsonParser`], so failures point at the exact offset that broke.
//!
//! # Features
//!
//! - Full JSON grammar: objects, arrays, strings, numbers, `true`/`false`/`null`
//! - String escapes including `\uXXXX` and surrogate pairs for astral characters
//! - Typed errors ([`JsonError`]) carrying the position and what was expected
//! - Serialization back to JSON via [`Display`](std::fmt::Display), or
//!   [`JsonValue::pretty_print`] for indented output
//! - Ergonomic access with [`JsonValue::get`], [`JsonValue::get_index`], and the
//!   `as_*` accessors
//! - Optional Python bindings behind the `python` feature
//!
//! # Example
//!
//! ```
//! use rust_json_parser::parse;
//!
//! let value = parse(r#"{"name": "Ferris", "langs": ["rust"]}"#)?;
//!
//! assert_eq!(value.get("name").and_then(|v| v.as_str()), Some("Ferris"));
//! assert_eq!(
//!     value.get("langs").and_then(|v| v.get_index(0)).and_then(|v| v.as_str()),
//!     Some("rust")
//! );
//! # Ok::<(), rust_json_parser::JsonError>(())
//! ```

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

#[cfg(feature = "python")]
mod python_bindings;
