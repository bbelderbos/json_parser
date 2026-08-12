use crate::error::{JsonError, Result};

#[derive(Debug, Clone, PartialEq, Default)]
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
    #[default]
    Null,
}

pub struct Tokenizer<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
        }
    }
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::with_capacity(self.input.len() / 4);

        while let Some(ch) = self.peek() {
            if let Some(token) = single_char_token(ch) {
                tokens.push(token);
                self.advance(); // consume the character
                continue;
            }
            match ch {
                b'"' => tokens.push(Token::String(self.read_string()?)),
                b'0'..=b'9' | b'-' => tokens.push(self.read_number()?),
                b't' | b'f' | b'n' => tokens.push(self.read_literal()?),
                b' ' | b'\n' | b'\r' | b'\t' => {
                    self.advance();
                }
                _ => {
                    return Err(JsonError::UnexpectedToken {
                        expected: "valid JSON token".to_string(),
                        found: char_at(self.input, self.position),
                        position: self.position,
                    });
                }
            }
        }
        Ok(tokens)
    }

    fn take_while(&mut self, predicate: impl Fn(u8) -> bool) -> &'a [u8] {
        let start = self.position;
        while self.peek().is_some_and(&predicate) {
            self.advance();
        }
        &self.input[start..self.position]
    }

    fn read_string(&mut self) -> Result<String> {
        let start = self.position;
        self.advance(); // consume the opening quote

        let content_start = self.position;
        while let Some(ch) = self.peek() {
            match ch {
                b'"' => {
                    let value = slice_to_string(&self.input[content_start..self.position]);
                    self.advance(); // consume the closing quote
                    return Ok(value);
                }
                b'\\' => return self.read_escaped_string(start, content_start),
                _ => {
                    self.advance();
                }
            }
        }
        Err(JsonError::UnterminatedString { position: start })
    }

    fn read_escaped_string(&mut self, start: usize, content_start: usize) -> Result<String> {
        let mut value = self.input[content_start..self.position].to_vec();
        loop {
            match self.peek() {
                None => return Err(JsonError::UnterminatedString { position: start }),
                Some(b'"') => {
                    self.advance(); // consume the closing quote
                    return Ok(slice_to_string(&value));
                }
                Some(b'\\') => {
                    self.advance(); // consume the backslash
                    let mut buf = [0u8; 4];
                    value.extend_from_slice(
                        self.read_escape(start)?.encode_utf8(&mut buf).as_bytes(),
                    );
                }
                Some(byte) => {
                    value.push(byte);
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
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{0008}',
            b'f' => '\u{000C}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => return self.parse_unicode_escape(),
            b'x' => return self.parse_hex_escape(),
            other => {
                return Err(JsonError::InvalidEscape {
                    char: other as char,
                    position: self.position,
                });
            }
        };
        self.advance(); // consume the escaped char
        Ok(escaped)
    }

    fn read_number(&mut self) -> Result<Token> {
        let start = self.position;
        let number_bytes = self.take_while(|b| b.is_ascii_digit() || b == b'.' || b == b'-');
        let number_str = std::str::from_utf8(number_bytes).unwrap_or_default();

        match number_str.parse::<f64>() {
            Ok(number) => Ok(Token::Number(number)),
            Err(_) => Err(JsonError::InvalidNumber {
                value: number_str.to_string(),
                position: start,
            }),
        }
    }

    fn read_literal(&mut self) -> Result<Token> {
        let start = self.position;
        let word = self.take_while(|b| b.is_ascii_alphabetic());

        match word {
            b"true" => Ok(Token::Boolean(true)),
            b"false" => Ok(Token::Boolean(false)),
            b"null" => Ok(Token::Null),
            _ => Err(JsonError::UnexpectedToken {
                expected: "true, false, or null".to_string(),
                found: slice_to_string(word),
                position: start,
            }),
        }
    }

    fn advance(&mut self) -> Option<u8> {
        if self.position < self.input.len() {
            let byte = self.input[self.position];
            self.position += 1;
            Some(byte)
        } else {
            None
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }

    fn read_hex_digits(&mut self, count: usize) -> Result<u32> {
        let start = self.position;
        let mut value = 0;
        for _ in 0..count {
            let Some(digit) = self.peek().and_then(|b| (b as char).to_digit(16)) else {
                return Err(JsonError::InvalidUnicode {
                    sequence: slice_to_string(&self.input[start..self.position]),
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

        if self.peek() != Some(b'\\') {
            return Err(invalid());
        }
        self.advance();

        if self.peek() != Some(b'u') {
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

fn single_char_token(byte: u8) -> Option<Token> {
    match byte {
        b'{' => Some(Token::LeftBrace),
        b'}' => Some(Token::RightBrace),
        b'[' => Some(Token::LeftBracket),
        b']' => Some(Token::RightBracket),
        b',' => Some(Token::Comma),
        b':' => Some(Token::Colon),
        _ => None,
    }
}

fn slice_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn char_at(bytes: &[u8], position: usize) -> String {
    let end = (position + 4).min(bytes.len());
    String::from_utf8_lossy(&bytes[position..end])
        .chars()
        .next()
        .map(String::from)
        .unwrap_or_default()
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
        // Can't directly inspect private fields,
        // so verify construction by using the struct's methods
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
    fn test_position_is_byte_index_after_multibyte() {
        // With a &[u8] cursor, position is a byte index: 'é' is 2 bytes.
        // '@' is at byte index 8 (0:" 1:c 2:a 3:f 4-5:é 6:" 7:space 8:@).
        let err = tokenize(r#""café" @"#).unwrap_err();
        assert_eq!(
            err,
            JsonError::UnexpectedToken {
                expected: "valid JSON token".to_string(),
                found: "@".to_string(),
                position: 8,
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
        assert_eq!(tokenizer.advance(), Some(b'a'));
        assert_eq!(tokenizer.advance(), Some(b'b'));
        assert_eq!(tokenizer.advance(), Some(b'c'));
        assert_eq!(tokenizer.advance(), None);
    }

    #[test]
    fn test_peek_doesnt_advance() {
        let mut tokenizer = Tokenizer::new("ab");
        assert_eq!(tokenizer.peek(), Some(b'a'));
        assert_eq!(tokenizer.peek(), Some(b'a'));
        assert_eq!(tokenizer.peek(), Some(b'a'));
        assert_eq!(tokenizer.advance(), Some(b'a'));
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
