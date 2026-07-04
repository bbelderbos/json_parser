use crate::error::JsonError;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Colon,
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
}

type Result<T> = std::result::Result<T, JsonError>;

pub fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::with_capacity(input.len() / 4);

    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '"' => {
                // todo: escape sequences, unicode, etc. is for later weeks
                let start_position = input.len() - chars.clone().count();
                chars.next();

                let mut string_value = String::new();
                let mut terminated = false;

                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '"' {
                        chars.next();
                        terminated = true;
                        break;
                    }
                    string_value.push(next_ch);
                    chars.next();
                }
                if !terminated {
                    return Err(JsonError::UnterminatedString {
                        position: start_position,
                    })?;
                }
                tokens.push(Token::String(string_value));
            }
            '{' | '}' | '[' | ']' | ',' | ':' => {
                let token = match ch {
                    '{' => Token::LeftBrace,
                    '}' => Token::RightBrace,
                    '[' => Token::LeftBracket,
                    ']' => Token::RightBracket,
                    ',' => Token::Comma,
                    ':' => Token::Colon,
                    _ => unreachable!(),
                };
                tokens.push(token);
                chars.next();
            }
            '0'..='9' | '-' => {
                let start_position = input.len() - chars.clone().count();
                let mut number_str = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_ascii_digit() || next_ch == '.' || next_ch == '-' {
                        number_str.push(next_ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Ok(number) = number_str.parse::<f64>() {
                    tokens.push(Token::Number(number));
                } else {
                    Err(JsonError::InvalidNumber {
                        value: number_str,
                        position: start_position,
                    })?;
                }
            }
            't' | 'f' | 'n' => {
                let start_position = input.len() - chars.clone().count();
                let mut temp_str = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphabetic() {
                        temp_str.push(next_ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                match temp_str.as_str() {
                    "true" => tokens.push(Token::Boolean(true)),
                    "false" => tokens.push(Token::Boolean(false)),
                    "null" => tokens.push(Token::Null),
                    _ => {
                        return Err(JsonError::UnexpectedToken {
                            expected: "true, false, or null".to_string(),
                            found: temp_str,
                            position: start_position,
                        })
                    }
                }
            }
            ' ' | '\n' | '\r' | '\t' => {
                // expected whitespace, skip it
                chars.next();
            }
            _ => {
                let current_position = input.len() - chars.clone().count();
                return Err(JsonError::UnexpectedToken {
                    expected: "valid JSON token".to_string(),
                    found: ch.to_string(),
                    position: current_position,
                });
            }
        }
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    type Result<T> = super::Result<T>;

    #[test]
    fn test_empty_braces() -> Result<()> {
        let tokens = tokenize("{}")?;
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], Token::LeftBrace);
        assert_eq!(tokens[1], Token::RightBrace);
        Ok(())
    }

    #[test]
    fn test_simple_string() -> Result<()> {
        let tokens = tokenize(r#""hello""#)?;
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::String("hello".to_string()));
        Ok(())
    }

    #[test]
    fn test_tokenize_string() -> Result<()> {
        let tokens = tokenize(r#""hello world""#)?;
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::String("hello world".to_string()));
        Ok(())
    }

    #[test]
    fn test_empty_string() -> Result<()> {
        // Outer boundary: adjacent quotes with no inner content
        let tokens = tokenize(r#""""#)?;
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::String("".to_string()));
        Ok(())
    }

    #[test]
    fn test_string_containing_json_special_chars() -> Result<()> {
        // Inner handling: JSON delimiters inside strings don't break tokenization
        let tokens = tokenize(r#""{key: value}""#)?;
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::String("{key: value}".to_string()));
        Ok(())
    }

    #[test]
    fn test_string_with_keyword_like_content() -> Result<()> {
        // Inner handling: "true", "false", "null" inside strings stay as string content
        let tokens = tokenize(r#""not true or false""#)?;
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::String("not true or false".to_string()));
        Ok(())
    }

    #[test]
    fn test_string_with_number_like_content() -> Result<()> {
        // Inner handling: numeric content inside strings doesn't become Number tokens
        let tokens = tokenize(r#""phone: 555-1234""#)?;
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::String("phone: 555-1234".to_string()));
        Ok(())
    }

    #[test]
    fn test_number() -> Result<()> {
        let tokens = tokenize("42")?;
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Number(42.0));
        Ok(())
    }

    #[test]
    fn test_negative_number() -> Result<()> {
        let tokens = tokenize("-42")?;
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Number(-42.0));
        Ok(())
    }

    #[test]
    fn test_decimal_number() -> Result<()> {
        let tokens = tokenize("0.5")?;
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Number(0.5));
        Ok(())
    }

    #[test]
    fn test_leading_decimal_not_a_number() {
        // .5 is invalid JSON - numbers must have leading digit (0.5 is valid)
        let err = tokenize(".5").unwrap_err();
        assert!(matches!(
            err,
            JsonError::UnexpectedToken { position: 0, .. }
        ));
    }

    #[test]
    fn test_malformed_numbers_silently_dropped() {
        let err = tokenize("1-2-3").unwrap_err();
        assert_eq!(
            err,
            JsonError::InvalidNumber {
                value: "1-2-3".to_string(),
                position: 0
            }
        );
    }

    #[test]
    fn test_boolean_and_null() -> Result<()> {
        let tokens = tokenize("true false null")?;
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::Boolean(true));
        assert_eq!(tokens[1], Token::Boolean(false));
        assert_eq!(tokens[2], Token::Null);
        Ok(())
    }

    #[test]
    fn test_simple_object() -> Result<()> {
        let tokens = tokenize(r#"{"name": "Alice"}"#)?;
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0], Token::LeftBrace);
        assert_eq!(tokens[1], Token::String("name".to_string()));
        assert_eq!(tokens[2], Token::Colon);
        assert_eq!(tokens[3], Token::String("Alice".to_string()));
        assert_eq!(tokens[4], Token::RightBrace);
        Ok(())
    }

    #[test]
    fn test_multiple_values() -> Result<()> {
        let tokens = tokenize(r#"{"age": 30, "active": true}"#)?;

        // Verify we have the right tokens
        assert!(tokens.contains(&Token::String("age".to_string())));
        assert!(tokens.contains(&Token::Number(30.0)));
        assert!(tokens.contains(&Token::Comma));
        assert!(tokens.contains(&Token::String("active".to_string())));
        assert!(tokens.contains(&Token::Boolean(true)));
        Ok(())
    }

    #[test]
    fn test_array() -> Result<()> {
        let tokens = tokenize(r#"[1, 2, 3]"#)?;
        assert_eq!(tokens.len(), 7);
        assert_eq!(tokens[0], Token::LeftBracket);
        assert_eq!(tokens[1], Token::Number(1.0));
        assert_eq!(tokens[2], Token::Comma);
        assert_eq!(tokens[3], Token::Number(2.0));
        assert_eq!(tokens[4], Token::Comma);
        assert_eq!(tokens[5], Token::Number(3.0));
        assert_eq!(tokens[6], Token::RightBracket);
        Ok(())
    }

    #[test]
    fn test_nested_objects() -> Result<()> {
        let tokens = tokenize(r#"{"person": {"name": "Alice", "age": 30}}"#)?;
        assert_eq!(tokens.len(), 13);
        assert_eq!(tokens[0], Token::LeftBrace);
        assert_eq!(tokens[1], Token::String("person".to_string()));
        assert_eq!(tokens[2], Token::Colon);
        assert_eq!(tokens[3], Token::LeftBrace);
        assert_eq!(tokens[4], Token::String("name".to_string()));
        assert_eq!(tokens[5], Token::Colon);
        assert_eq!(tokens[6], Token::String("Alice".to_string()));
        assert_eq!(tokens[7], Token::Comma);
        assert_eq!(tokens[8], Token::String("age".to_string()));
        assert_eq!(tokens[9], Token::Colon);
        assert_eq!(tokens[10], Token::Number(30.0));
        assert_eq!(tokens[11], Token::RightBrace);
        assert_eq!(tokens[12], Token::RightBrace);
        Ok(())
    }

    #[test]
    fn test_edge_cases() -> Result<()> {
        let tokens = tokenize(r#""", 0, -5"#)?;
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0], Token::String("".to_string()));
        assert_eq!(tokens[1], Token::Comma);
        assert_eq!(tokens[2], Token::Number(0.0));
        assert_eq!(tokens[3], Token::Comma);
        assert_eq!(tokens[4], Token::Number(-5.0));
        Ok(())
    }

    #[test]
    fn test_sample_json_file() -> Result<()> {
        let input = include_str!("../../test_data/sample.json");
        let tokens = tokenize(input)?;

        assert!(tokens.contains(&Token::String("name".to_string())));
        assert!(tokens.contains(&Token::String("Alice Johnson".to_string())));
        assert!(tokens.contains(&Token::Number(28.0)));
        assert!(tokens.contains(&Token::Boolean(true)));
        assert!(tokens.contains(&Token::String("tags".to_string())));
        assert!(tokens.contains(&Token::LeftBracket));
        assert!(tokens.contains(&Token::String("developer".to_string())));
        Ok(())
    }

    #[test]
    fn test_unterminated_string_should_not_produce_token() {
        let err = tokenize(r#""hello"#).unwrap_err();
        assert!(matches!(err, JsonError::UnterminatedString { position: 0 }))
    }
}
