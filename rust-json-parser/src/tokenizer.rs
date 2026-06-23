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

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();

    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '"' => {
                chars.next();
                let mut string_value = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '"' {
                        chars.next();
                        break;
                    }
                    string_value.push(next_ch);
                    chars.next();
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
                    if next_ch.is_digit(10) || next_ch == '.' || next_ch == '-' {
                        number_str.push(next_ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Ok(number) = number_str.parse::<f64>() {
                    tokens.push(Token::Number(number));
                } else {
                    println!("Invalid number: {}", number_str);
                }
            }
            't' | 'f' => {
                let mut bool_str = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphabetic() {
                        bool_str.push(next_ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                match bool_str.as_str() {
                    "true" => tokens.push(Token::Boolean(true)),
                    "false" => tokens.push(Token::Boolean(false)),
                    _ => println!("Unexpected token: {}", bool_str),
                }
            }
            'n' => {
                let mut null_str = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphabetic() {
                        null_str.push(next_ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if null_str == "null" {
                    tokens.push(Token::Null);
                } else {
                    println!("Unexpected token: {}", null_str);
                }
            }
            ' ' | '\n' | '\r' | '\t' => {
                // expected whitespace, skip it
                chars.next();
            }
            _ => {
                println!("Unexpected character: {}", ch);
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
        // .5 is invalid JSON - numbers must have leading digit (0.5 is valid)
        let tokens = tokenize(".5");
        // Should NOT be interpreted as 0.5
        assert!(!tokens.contains(&Token::Number(0.5)));
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
        println!("{:?}", tokens);
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
}
