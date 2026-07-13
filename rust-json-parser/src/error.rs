use std::fmt;

pub type Result<T> = std::result::Result<T, JsonError>;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonError {
    UnexpectedToken {
        expected: String,
        found: String,
        position: usize,
    },
    UnexpectedEndOfInput {
        expected: String,
        position: usize,
    },
    InvalidNumber {
        value: String,
        position: usize,
    },
    UnterminatedString {
        position: usize,
    },
    InvalidEscape {
        char: char,
        position: usize,
    },
    InvalidUnicode {
        sequence: String,
        position: usize,
    },
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonError::UnexpectedToken {
                expected,
                found,
                position,
            } => {
                write!(
                    f,
                    "Unexpected token at position {position}: expected '{expected}', found '{found}'"
                )
            }
            JsonError::UnexpectedEndOfInput { expected, position } => {
                write!(
                    f,
                    "Unexpected end of input at position {position}: expected '{expected}'"
                )
            }
            JsonError::InvalidNumber { value, position } => {
                write!(f, "Invalid number '{value}' at position {position}")
            }
            JsonError::UnterminatedString { position } => {
                write!(f, "Unterminated string starting at position {position}")
            }
            JsonError::InvalidEscape { char, position } => {
                write!(
                    f,
                    "Invalid escape character '{char}' at position {position}"
                )
            }
            JsonError::InvalidUnicode { sequence, position } => {
                write!(
                    f,
                    "Invalid unicode escape sequence '{sequence}' at position {position}"
                )
            }
        }
    }
}

impl std::error::Error for JsonError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = JsonError::UnexpectedToken {
            expected: "number".to_string(),
            found: "@".to_string(),
            position: 5,
        };

        // Error should be Debug-printable
        assert!(format!("{:?}", error).contains("UnexpectedToken"));
    }

    #[test]
    fn test_error_variants() {
        let token_error = JsonError::UnexpectedToken {
            expected: "number".to_string(),
            found: "x".to_string(),
            position: 3,
        };

        let eof_error = JsonError::UnexpectedEndOfInput {
            expected: "closing quote".to_string(),
            position: 10,
        };

        let num_error = JsonError::InvalidNumber {
            value: "12.34.56".to_string(),
            position: 0,
        };

        let unterminated_string_error = JsonError::UnterminatedString { position: 15 };
        let invalid_escape_error = JsonError::InvalidEscape {
            char: 'q',
            position: 20,
        };
        let invalid_unicode_error = JsonError::InvalidUnicode {
            sequence: "\\uZZZZ".to_string(),
            position: 25,
        };
        // All variants should be Debug-printable
        assert!(!format!("{:?}", token_error).is_empty());
        assert!(!format!("{:?}", eof_error).is_empty());
        assert!(!format!("{:?}", num_error).is_empty());
        assert!(!format!("{:?}", unterminated_string_error).is_empty());
        assert!(!format!("{:?}", invalid_escape_error).is_empty());
        assert!(!format!("{:?}", invalid_unicode_error).is_empty());
    }

    #[test]
    fn test_error_display() {
        let error = JsonError::UnexpectedToken {
            expected: "valid JSON".to_string(),
            found: "@".to_string(),
            position: 0,
        };

        let message = format!("{}", error);
        assert!(message.contains("position 0"));
        assert!(message.contains("valid JSON"));
        assert!(message.contains("@"));
    }

    #[test]
    fn test_invalid_escape_display() {
        let err = JsonError::InvalidEscape {
            char: 'q',
            position: 5,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("escape"));
        assert!(msg.contains("q"));
    }

    #[test]
    fn test_invalid_unicode_display() {
        let err = JsonError::InvalidUnicode {
            sequence: "00GG".to_string(),
            position: 3,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("unicode") || msg.contains("Unicode"));
    }

    #[test]
    fn test_error_is_std_error() {
        let err = JsonError::InvalidEscape {
            char: 'x',
            position: 0,
        };
        let _: &dyn std::error::Error = &err; // Must implement Error trait
    }
}
