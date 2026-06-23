#[derive(Debug, PartialEq)]
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

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::with_capacity(input.len() / 4);

    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '"' => {
                // todo: escape sequences, unicode, etc. is for later weeks
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
                    eprintln!("Unterminated string literal");
                    break;
                }
                tokens.push(Token::String(string_value));
            }
            '{' => {
                tokens.push(Token::LeftBrace);
                chars.next();
            }
            '}' => {
                tokens.push(Token::RightBrace);
                chars.next();
            }
            '[' => {
                tokens.push(Token::LeftBracket);
                chars.next();
            }
            ']' => {
                tokens.push(Token::RightBracket);
                chars.next();
            }
            ',' => {
                tokens.push(Token::Comma);
                chars.next();
            }
            ':' => {
                tokens.push(Token::Colon);
                chars.next();
            }
            '0'..='9' | '-' => {
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
                    // todo: handle error
                    eprintln!("Invalid number: {}", number_str);
                }
            }
            't' | 'f' | 'n' => {
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
                    // todo: handle error
                    _ => eprintln!("Unexpected token: {}", temp_str),
                }
            }
            ' ' | '\n' | '\r' | '\t' => {
                // expected whitespace, skip it
                chars.next();
            }
            _ => {
                // todo: handle error
                eprintln!("Unexpected character: {}", ch);
                chars.next();
            }
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_braces() {
        let tokens = tokenize("{}");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], Token::LeftBrace);
        assert_eq!(tokens[1], Token::RightBrace);
    }

    #[test]
    fn test_simple_string() {
        let tokens = tokenize(r#""hello""#);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::String("hello".to_string()));
    }

    #[test]
    fn test_tokenize_string() {
        let tokens = tokenize(r#""hello world""#);

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::String("hello world".to_string()));
    }

    #[test]
    fn test_empty_string() {
        // Outer boundary: adjacent quotes with no inner content
        let tokens = tokenize(r#""""#);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::String("".to_string()));
    }

    #[test]
    fn test_string_containing_json_special_chars() {
        // Inner handling: JSON delimiters inside strings don't break tokenization
        let tokens = tokenize(r#""{key: value}""#);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::String("{key: value}".to_string()));
    }

    #[test]
    fn test_string_with_keyword_like_content() {
        // Inner handling: "true", "false", "null" inside strings stay as string content
        let tokens = tokenize(r#""not true or false""#);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::String("not true or false".to_string()));
    }

    #[test]
    fn test_string_with_number_like_content() {
        // Inner handling: numeric content inside strings doesn't become Number tokens
        let tokens = tokenize(r#""phone: 555-1234""#);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::String("phone: 555-1234".to_string()));
    }

    #[test]
    fn test_number() {
        let tokens = tokenize("42");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Number(42.0));
    }

    #[test]
    fn test_negative_number() {
        let tokens = tokenize("-42");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Number(-42.0));
    }

    #[test]
    fn test_decimal_number() {
        let tokens = tokenize("0.5");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Number(0.5));
    }

    #[test]
    fn test_leading_decimal_not_a_number() {
        // .5 is invalid JSON, currently it skips the leading dot and treats it as a number starting with 5
        // this is not correct json though
        let tokens = tokenize(".5");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Number(5.0)); // todo: tokenizer to reject leading decimal without a leading zero
    }

    #[test]
    fn test_malformed_numbers_silently_dropped() {
        let tokens = tokenize("1-2-3");
        assert!(tokens.is_empty());

        let tokens = tokenize("1.2.3");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_boolean_and_null() {
        let tokens = tokenize("true false null");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::Boolean(true));
        assert_eq!(tokens[1], Token::Boolean(false));
        assert_eq!(tokens[2], Token::Null);
    }

    #[test]
    fn test_simple_object() {
        let tokens = tokenize(r#"{"name": "Alice"}"#);
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0], Token::LeftBrace);
        assert_eq!(tokens[1], Token::String("name".to_string()));
        assert_eq!(tokens[2], Token::Colon);
        assert_eq!(tokens[3], Token::String("Alice".to_string()));
        assert_eq!(tokens[4], Token::RightBrace);
    }

    #[test]
    fn test_multiple_values() {
        let tokens = tokenize(r#"{"age": 30, "active": true}"#);

        // Verify we have the right tokens
        assert!(tokens.contains(&Token::String("age".to_string())));
        assert!(tokens.contains(&Token::Number(30.0)));
        assert!(tokens.contains(&Token::Comma));
        assert!(tokens.contains(&Token::String("active".to_string())));
        assert!(tokens.contains(&Token::Boolean(true)));
    }

    #[test]
    fn test_array() {
        let tokens = tokenize(r#"[1, 2, 3]"#);
        assert_eq!(tokens.len(), 7);
        assert_eq!(tokens[0], Token::LeftBracket);
        assert_eq!(tokens[1], Token::Number(1.0));
        assert_eq!(tokens[2], Token::Comma);
        assert_eq!(tokens[3], Token::Number(2.0));
        assert_eq!(tokens[4], Token::Comma);
        assert_eq!(tokens[5], Token::Number(3.0));
        assert_eq!(tokens[6], Token::RightBracket);
    }

    #[test]
    fn test_nested_objects() {
        let tokens = tokenize(r#"{"person": {"name": "Alice", "age": 30}}"#);
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
    }

    #[test]
    fn test_edge_cases() {
        let tokens = tokenize(r#""", 0, -5"#);
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0], Token::String("".to_string()));
        assert_eq!(tokens[1], Token::Comma);
        assert_eq!(tokens[2], Token::Number(0.0));
        assert_eq!(tokens[3], Token::Comma);
        assert_eq!(tokens[4], Token::Number(-5.0));
    }

    #[test]
    fn test_sample_json_file() {
        let input = include_str!("../../test_data/sample.json");
        let tokens = tokenize(input);

        assert!(tokens.contains(&Token::String("name".to_string())));
        assert!(tokens.contains(&Token::String("Alice Johnson".to_string())));
        assert!(tokens.contains(&Token::Number(28.0)));
        assert!(tokens.contains(&Token::Boolean(true)));
        assert!(tokens.contains(&Token::String("tags".to_string())));
        assert!(tokens.contains(&Token::LeftBracket));
        assert!(tokens.contains(&Token::String("developer".to_string())));
    }

    #[test]
    fn test_unterminated_string_should_not_produce_token() {
        let tokens = tokenize(r#""hello"#);
        assert!(
            tokens.is_empty(),
            "unterminated string should not emit a token, got: {:?}",
            tokens
        );
    }
}
