use crate::error::{JsonError, Result};

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

pub struct Tokenizer {
    input: Vec<char>,
    position: usize,
}

impl Tokenizer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
        }
    }
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::with_capacity(self.input.len() / 4);

        while let Some(ch) = self.peek() {
            match ch {
                '"' => tokens.push(Token::String(self.read_string()?)),
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
                    self.advance(); // consume the character
                }
                '0'..='9' | '-' => tokens.push(self.read_number()?),
                't' | 'f' | 'n' => tokens.push(self.read_literal()?),
                ' ' | '\n' | '\r' | '\t' => {
                    self.advance();
                }
                _ => {
                    return Err(JsonError::UnexpectedToken {
                        expected: "valid JSON token".to_string(),
                        found: ch.to_string(),
                        position: self.position,
                    });
                }
            }
        }
        Ok(tokens)
    }

    fn take_while(&mut self, predicate: impl Fn(char) -> bool) -> String {
        let mut taken = String::new();
        while let Some(ch) = self.peek().filter(|&ch| predicate(ch)) {
            taken.push(ch);
            self.advance();
        }
        taken
    }

    fn read_string(&mut self) -> Result<String> {
        let start = self.position;
        self.advance(); // consume the opening quote

        let mut value = String::new();
        loop {
            match self.peek() {
                None => return Err(JsonError::UnterminatedString { position: start }),
                Some('"') => {
                    self.advance(); // consume the closing quote
                    return Ok(value);
                }
                Some('\\') => {
                    self.advance(); // consume the backslash
                    value.push(self.read_escape(start)?);
                }
                Some(ch) => {
                    value.push(ch);
                    self.advance();
                }
            }
        }
    }

    fn read_escape(&mut self, string_start: usize) -> Result<char> {
        let Some(ch) = self.peek() else {
            return Err(JsonError::UnterminatedString {
                position: string_start,
            });
        };
        let escaped = match ch {
            '"' => '"',
            '\\' => '\\',
            '/' => '/',
            'b' => '\u{0008}',
            'f' => '\u{000C}',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'u' => return self.parse_unicode_escape(),
            'x' => return self.parse_hex_escape(),
            other => {
                return Err(JsonError::InvalidEscape {
                    char: other,
                    position: self.position,
                });
            }
        };
        self.advance(); // consume the escaped char
        Ok(escaped)
    }

    fn read_number(&mut self) -> Result<Token> {
        let start = self.position;
        let number_str = self.take_while(|ch| ch.is_ascii_digit() || ch == '.' || ch == '-');

        match number_str.parse::<f64>() {
            Ok(number) => Ok(Token::Number(number)),
            Err(_) => Err(JsonError::InvalidNumber {
                value: number_str,
                position: start,
            }),
        }
    }

    fn read_literal(&mut self) -> Result<Token> {
        let start = self.position;
        let word = self.take_while(char::is_alphabetic);

        match word.as_str() {
            "true" => Ok(Token::Boolean(true)),
            "false" => Ok(Token::Boolean(false)),
            "null" => Ok(Token::Null),
            _ => Err(JsonError::UnexpectedToken {
                expected: "true, false, or null".to_string(),
                found: word,
                position: start,
            }),
        }
    }

    fn advance(&mut self) -> Option<char> {
        if self.position < self.input.len() {
            let ch = self.input[self.position];
            self.position += 1;
            Some(ch)
        } else {
            None
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.position).cloned()
    }

    fn read_hex_digits(&mut self, count: usize) -> Result<u32> {
        let start = self.position;
        let mut value = 0;
        for _ in 0..count {
            let Some(digit) = self.peek().and_then(|ch| ch.to_digit(16)) else {
                return Err(JsonError::InvalidUnicode {
                    sequence: self.input[start..self.position].iter().collect(),
                    position: self.position,
                });
            };
            value = value * 16 + digit;
            self.advance();
        }
        Ok(value)
    }

    fn parse_unicode_escape(&mut self) -> Result<char> {
        self.advance(); // consume the 'u'
        let first = self.read_hex_digits(4)?;

        let code_point = if is_high_surrogate(first) {
            let low = self.parse_low_surrogate(first)?;
            combine_surrogates(first, low)
        } else {
            first
        };

        std::char::from_u32(code_point).ok_or(JsonError::InvalidUnicode {
            sequence: format!("{code_point:04X}"),
            position: self.position,
        })
    }

    fn parse_low_surrogate(&mut self, high: u32) -> Result<u32> {
        let position = self.position;
        let invalid = || JsonError::InvalidUnicode {
            sequence: format!("{high:04X}"),
            position,
        };

        if self.peek() != Some('\\') {
            return Err(invalid());
        }
        self.advance();

        if self.peek() != Some('u') {
            return Err(invalid());
        }
        self.advance();

        let low = self.read_hex_digits(4)?;
        if is_low_surrogate(low) {
            Ok(low)
        } else {
            Err(invalid())
        }
    }

    fn parse_hex_escape(&mut self) -> Result<char> {
        self.advance(); // consume the 'x'
        let byte = self.read_hex_digits(2)?;
        Ok(char::from(byte as u8))
    }
}

fn is_high_surrogate(code_point: u32) -> bool {
    (0xD800..=0xDBFF).contains(&code_point)
}

fn is_low_surrogate(code_point: u32) -> bool {
    (0xDC00..=0xDFFF).contains(&code_point)
}

fn combine_surrogates(high: u32, low: u32) -> u32 {
    0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(input: &str) -> Result<Vec<Token>> {
        let mut tokenizer = Tokenizer::new(input);
        tokenizer.tokenize()
    }

    #[test]
    fn test_tokenizer_new() -> Result<()> {
        let _tokenizer = Tokenizer::new("hello");
        // Can't directly inspect private fields
        // Verify by using the struct's methods
        let mut t = Tokenizer::new("42");
        let tokens = t.tokenize()?;
        assert_eq!(tokens.len(), 1);
        Ok(())
    }

    #[test]
    fn test_initial_position() {
        let tokenizer = Tokenizer::new("test");
        assert_eq!(tokenizer.position, 0);
    }

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
    fn test_malformed_numbers_rejected() {
        for value in ["1-2-3", "1.2.3"] {
            let err = tokenize(value).unwrap_err();
            assert_eq!(
                err,
                JsonError::InvalidNumber {
                    value: value.to_string(),
                    position: 0
                }
            );
        }
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
    fn test_position_is_char_index_after_multibyte() {
        // With a Vec<char> cursor, position is a char index: 'é' counts as one.
        // '@' is the 8th char (index 7), regardless of 'é' being 2 bytes.
        let err = tokenize(r#""café" @"#).unwrap_err();
        assert_eq!(
            err,
            JsonError::UnexpectedToken {
                expected: "valid JSON token".to_string(),
                found: "@".to_string(),
                position: 7,
            }
        );
    }

    #[test]
    fn test_unterminated_string_should_not_produce_token() {
        let err = tokenize(r#""hello"#).unwrap_err();
        assert!(matches!(err, JsonError::UnterminatedString { position: 0 }))
    }

    #[test]
    fn test_tokenize_literals() -> Result<()> {
        assert_eq!(tokenize("true")?, vec![Token::Boolean(true)]);
        assert_eq!(tokenize("false")?, vec![Token::Boolean(false)]);
        assert_eq!(tokenize("null")?, vec![Token::Null]);
        Ok(())
    }

    #[test]
    fn test_tokenizer_multiple_tokens() -> Result<()> {
        // Tests that a single tokenize() call handles multiple tokens
        // Note: Unlike Python iterators, calling tokenize() again on the same
        // instance would return empty - the input has been consumed.
        // Create a new Tokenizer instance if you need to parse new input.
        let tokens = tokenize("123 456")?;
        assert_eq!(tokens.len(), 2);
        Ok(())
    }

    #[test]
    fn test_tokenize_negative_number() -> Result<()> {
        let tokens = tokenize("-3.5")?;
        assert_eq!(tokens, vec![Token::Number(-3.5)]);
        Ok(())
    }

    #[test]
    fn test_invalid_keyword_error_position_points_to_start() {
        let err = tokenize("   xyz").unwrap_err();
        match err {
            JsonError::UnexpectedToken { position, .. } => {
                assert_eq!(
                    position, 3,
                    "error position should point to the start of 'xyz' (index 3), not past it"
                );
            }
            other => panic!("expected UnexpectedToken, got {:?}", other),
        }
    }

    #[test]
    fn test_escape_newline() -> Result<()> {
        let tokens = tokenize(r#""hello\nworld""#)?;
        assert_eq!(tokens, vec![Token::String("hello\nworld".to_string())]);
        Ok(())
    }

    #[test]
    fn test_escape_tab() -> Result<()> {
        let tokens = tokenize(r#""col1\tcol2""#)?;
        assert_eq!(tokens, vec![Token::String("col1\tcol2".to_string())]);
        Ok(())
    }

    #[test]
    fn test_escape_quote() -> Result<()> {
        let tokens = tokenize(r#""say \"hello\"""#)?;
        assert_eq!(tokens, vec![Token::String("say \"hello\"".to_string())]);
        Ok(())
    }

    #[test]
    fn test_escape_backslash() -> Result<()> {
        let tokens = tokenize(r#""path\\to\\file""#)?;
        assert_eq!(tokens, vec![Token::String("path\\to\\file".to_string())]);
        Ok(())
    }

    #[test]
    fn test_multiple_escapes() -> Result<()> {
        let tokens = tokenize(r#""a\nb\tc\"""#)?;
        assert_eq!(tokens, vec![Token::String("a\nb\tc\"".to_string())]);
        Ok(())
    }

    #[test]
    fn test_escape_forward_slash() -> Result<()> {
        let tokens = tokenize(r#""a\/b""#)?;
        assert_eq!(tokens, vec![Token::String("a/b".to_string())]);
        Ok(())
    }

    #[test]
    fn test_escape_carriage_return() -> Result<()> {
        let tokens = tokenize(r#""line\r\n""#)?;
        assert_eq!(tokens, vec![Token::String("line\r\n".to_string())]);
        Ok(())
    }

    #[test]
    fn test_escape_backspace_formfeed() -> Result<()> {
        let tokens = tokenize(r#""\b\f""#)?;
        assert_eq!(tokens, vec![Token::String("\u{0008}\u{000C}".to_string())]);
        Ok(())
    }

    #[test]
    fn test_unicode_escape_basic() -> Result<()> {
        // \u0041 is 'A'
        let tokens = tokenize(r#""\u0041""#)?;
        assert_eq!(tokens, vec![Token::String("A".to_string())]);
        Ok(())
    }

    #[test]
    fn test_unicode_escape_multiple() -> Result<()> {
        // \u0048\u0069 is "Hi"
        let tokens = tokenize(r#""\u0048\u0069""#)?;
        assert_eq!(tokens, vec![Token::String("Hi".to_string())]);
        Ok(())
    }

    #[test]
    fn test_unicode_escape_mixed() -> Result<()> {
        // Mix of regular chars and unicode escapes
        let tokens = tokenize(r#""Hello \u0057orld""#)?;
        assert_eq!(tokens, vec![Token::String("Hello World".to_string())]);
        Ok(())
    }

    #[test]
    fn test_unicode_escape_lowercase() -> Result<()> {
        // Lowercase hex digits should work too
        let tokens = tokenize(r#""\u004a""#)?;
        assert_eq!(tokens, vec![Token::String("J".to_string())]);
        Ok(())
    }

    #[test]
    fn test_unicode_escape_surrogate_pair() -> Result<()> {
        // \uD83D\uDE00 is the emoji, built from a high and low surrogate
        let tokens = tokenize(r#""\uD83D\uDE00""#)?;
        assert_eq!(tokens, vec![Token::String("😀".to_string())]);
        Ok(())
    }

    #[test]
    fn test_unicode_escape_surrogate_pair_mixed() -> Result<()> {
        let tokens = tokenize(r#""hi \uD83D\uDE00!""#)?;
        assert_eq!(tokens, vec![Token::String("hi 😀!".to_string())]);
        Ok(())
    }

    #[test]
    fn test_lone_high_surrogate_rejected() {
        let result = tokenize(r#""\uD83D""#);
        assert!(matches!(result, Err(JsonError::InvalidUnicode { .. })));
    }

    #[test]
    fn test_lone_low_surrogate_rejected() {
        let result = tokenize(r#""\uDE00""#);
        assert!(matches!(result, Err(JsonError::InvalidUnicode { .. })));
    }

    #[test]
    fn test_high_surrogate_followed_by_non_surrogate_rejected() {
        let result = tokenize(r#""\uD83DA""#);
        assert!(matches!(result, Err(JsonError::InvalidUnicode { .. })));
    }

    #[test]
    fn test_high_surrogate_followed_by_plain_char_rejected() {
        let result = tokenize(r#""\uD83Dx""#);
        assert!(matches!(result, Err(JsonError::InvalidUnicode { .. })));
    }

    #[test]
    fn test_hex_escape_basic() -> Result<()> {
        // \x41 is 'A'
        let tokens = tokenize(r#""\x41""#)?;
        assert_eq!(tokens, vec![Token::String("A".to_string())]);
        Ok(())
    }

    #[test]
    fn test_hex_escape_lowercase() -> Result<()> {
        let tokens = tokenize(r#""\x4a""#)?;
        assert_eq!(tokens, vec![Token::String("J".to_string())]);
        Ok(())
    }

    #[test]
    fn test_hex_escape_mixed() -> Result<()> {
        let tokens = tokenize(r#""a\x42c""#)?;
        assert_eq!(tokens, vec![Token::String("aBc".to_string())]);
        Ok(())
    }

    #[test]
    fn test_hex_escape_high_byte() -> Result<()> {
        // \xFF is 'ÿ' - the top of the latin-1 range
        let tokens = tokenize(r#""\xFF""#)?;
        assert_eq!(tokens, vec![Token::String("ÿ".to_string())]);
        Ok(())
    }

    #[test]
    fn test_hex_escape_too_short() {
        let result = tokenize(r#""\x4""#);
        assert!(matches!(result, Err(JsonError::InvalidUnicode { .. })));
    }

    #[test]
    fn test_hex_escape_bad_hex() {
        let result = tokenize(r#""\xGG""#);
        assert!(matches!(result, Err(JsonError::InvalidUnicode { .. })));
    }

    #[test]
    fn test_invalid_escape_sequence() {
        let result = tokenize(r#""\q""#);
        assert!(matches!(result, Err(JsonError::InvalidEscape { .. })));
    }

    #[test]
    fn test_invalid_unicode_too_short() {
        let result = tokenize(r#""\u004""#);
        assert!(matches!(result, Err(JsonError::InvalidUnicode { .. })));
    }

    #[test]
    fn test_invalid_unicode_reports_digits_read_so_far() {
        // The reported sequence is the raw partial digits, so leading zeros survive
        for (input, sequence, position) in [
            (r#""\u00GG""#, "00", 5),
            (r#""\uGG""#, "", 3),
            (r#""\u004""#, "004", 6),
            (r#""\x4""#, "4", 4),
            (r#""\xGG""#, "", 3),
        ] {
            assert_eq!(
                tokenize(input).unwrap_err(),
                JsonError::InvalidUnicode {
                    sequence: sequence.to_string(),
                    position,
                },
                "for input {input}"
            );
        }
    }

    #[test]
    fn test_invalid_unicode_bad_hex() {
        let result = tokenize(r#""\u00GG""#);
        assert!(matches!(result, Err(JsonError::InvalidUnicode { .. })));
    }

    #[test]
    fn test_unterminated_string_with_escape() {
        let result = tokenize(r#""hello\n"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_advance_sequence() {
        let mut tokenizer = Tokenizer::new("abc");
        assert_eq!(tokenizer.advance(), Some('a'));
        assert_eq!(tokenizer.advance(), Some('b'));
        assert_eq!(tokenizer.advance(), Some('c'));
        assert_eq!(tokenizer.advance(), None);
    }

    #[test]
    fn test_peek_doesnt_advance() {
        let mut tokenizer = Tokenizer::new("ab");
        assert_eq!(tokenizer.peek(), Some('a'));
        assert_eq!(tokenizer.peek(), Some('a'));
        assert_eq!(tokenizer.peek(), Some('a'));
        assert_eq!(tokenizer.advance(), Some('a'));
    }

    #[test]
    fn test_advance_order_matters() {
        let mut t1 = Tokenizer::new("12");
        let mut t2 = Tokenizer::new("12");

        // Same operations in same order
        assert_eq!(t1.advance(), t2.advance());
        assert_eq!(t1.advance(), t2.advance());
    }

    #[test]
    fn test_invalid_escape_contains_char() {
        let result = tokenize(r#""\q""#);

        match result {
            Err(JsonError::InvalidEscape { char, position }) => {
                assert_eq!(char, 'q');
                assert!(position > 0); // Not at start
            }
            _ => panic!("Expected InvalidEscape error"),
        }
    }

    #[test]
    fn test_error_message_is_helpful() {
        let err = JsonError::InvalidEscape {
            char: 'x',
            position: 5,
        };
        let msg = format!("{}", err);

        assert!(msg.contains("escape"), "Should mention 'escape'");
        assert!(msg.contains("x"), "Should include the invalid char");
    }
}
