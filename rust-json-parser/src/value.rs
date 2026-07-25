use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        match self {
            JsonValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn pretty_print(&self, indent: usize) -> String {
        let mut result = String::new();
        let _ = write_json(self, &mut result, Some(indent), 0);
        result
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(obj) => obj.get(key),
            _ => None,
        }
    }

    pub fn get_index(&self, index: usize) -> Option<&JsonValue> {
        match self {
            JsonValue::Array(arr) => arr.get(index),
            _ => None,
        }
    }
}

fn write_json<W: fmt::Write>(
    value: &JsonValue,
    f: &mut W,
    indent: Option<usize>,
    current: usize,
) -> fmt::Result {
    let inner = current + indent.unwrap_or(0);
    let (nl, pad, close) = match indent {
        Some(_) => ("\n", " ".repeat(inner), " ".repeat(current)),
        None => ("", String::new(), String::new()),
    };

    match value {
        JsonValue::Null => f.write_str("null"),
        JsonValue::Boolean(b) => write!(f, "{b}"),
        JsonValue::Number(n) => write!(f, "{n}"),
        JsonValue::String(s) => write_json_string(f, s),
        JsonValue::Array(arr) => {
            f.write_char('[')?;
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    f.write_char(',')?;
                }
                write!(f, "{nl}{pad}")?;
                write_json(v, f, indent, inner)?;
            }
            if !arr.is_empty() {
                write!(f, "{nl}{close}")?;
            }
            f.write_char(']')
        }
        JsonValue::Object(obj) => {
            f.write_char('{')?;
            for (i, (k, v)) in obj.iter().enumerate() {
                if i > 0 {
                    f.write_char(',')?;
                }
                write!(f, "{nl}{pad}")?;
                write_json_string(f, k)?;
                f.write_char(':')?;
                write_json(v, f, indent, inner)?;
            }
            if !obj.is_empty() {
                write!(f, "{nl}{close}")?;
            }
            f.write_char('}')
        }
    }
}

fn write_json_string<W: fmt::Write>(f: &mut W, s: &str) -> fmt::Result {
    write!(f, "\"")?;
    for c in s.chars() {
        match c {
            '"' => write!(f, "\\\"")?,
            '\\' => write!(f, "\\\\")?,
            '\n' => write!(f, "\\n")?,
            '\r' => write!(f, "\\r")?,
            '\t' => write!(f, "\\t")?,
            '\u{08}' => write!(f, "\\b")?,
            '\u{0C}' => write!(f, "\\f")?,
            c if (c as u32) < 0x20 => write!(f, "\\u{:04x}", c as u32)?,
            c => write!(f, "{c}")?,
        }
    }
    write!(f, "\"")
}

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_json(self, f, None, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JsonParser;
    use crate::error::Result;

    #[test]
    fn test_json_value_equality() {
        assert_eq!(JsonValue::Null, JsonValue::Null);
        assert_eq!(JsonValue::Boolean(true), JsonValue::Boolean(true));
        assert_eq!(JsonValue::Number(42.0), JsonValue::Number(42.0));
        assert_eq!(
            JsonValue::String("test".to_string()),
            JsonValue::String("test".to_string())
        );

        assert_ne!(JsonValue::Null, JsonValue::Boolean(false));
        assert_ne!(JsonValue::Number(1.0), JsonValue::Number(2.0));
    }

    #[test]
    fn test_json_value_creation() {
        let null_val = JsonValue::Null;
        let bool_val = JsonValue::Boolean(true);
        let num_val = JsonValue::Number(42.5);
        let str_val = JsonValue::String("hello".to_string());

        assert!(null_val.is_null());
        assert_eq!(bool_val.as_bool(), Some(true));
        assert_eq!(num_val.as_f64(), Some(42.5));
        assert_eq!(str_val.as_str(), Some("hello"));
    }

    #[test]
    fn test_json_value_accessors() {
        let value = JsonValue::String("test".to_string());
        assert_eq!(value.as_str(), Some("test"));
        assert_eq!(value.as_f64(), None);
        assert_eq!(value.as_bool(), None);
        assert!(!value.is_null());

        let value = JsonValue::Number(42.0);
        assert_eq!(value.as_f64(), Some(42.0));
        assert_eq!(value.as_str(), None);

        let value = JsonValue::Boolean(true);
        assert_eq!(value.as_bool(), Some(true));

        let value = JsonValue::Null;
        assert!(value.is_null());
    }

    #[test]
    fn test_display_primitives() {
        assert_eq!(JsonValue::Null.to_string(), "null");
        assert_eq!(JsonValue::Boolean(true).to_string(), "true");
        assert_eq!(JsonValue::Boolean(false).to_string(), "false");
        assert_eq!(JsonValue::Number(42.0).to_string(), "42");
        assert_eq!(JsonValue::Number(3.14).to_string(), "3.14");
        assert_eq!(
            JsonValue::String("hello".to_string()).to_string(),
            "\"hello\""
        );
    }

    #[test]
    fn test_display_array() {
        let value = JsonValue::Array(vec![JsonValue::Number(1.0), JsonValue::Number(2.0)]);
        assert_eq!(value.to_string(), "[1,2]");
    }

    #[test]
    fn test_display_empty_containers() {
        assert_eq!(JsonValue::Array(vec![]).to_string(), "[]");
        assert_eq!(JsonValue::Object(HashMap::new()).to_string(), "{}");
    }

    #[test]
    fn test_display_escape_string() {
        let value = JsonValue::String("hello\nworld".to_string());
        assert_eq!(value.to_string(), "\"hello\\nworld\"");
    }

    #[test]
    fn test_display_escape_quotes() {
        let value = JsonValue::String("say \"hi\"".to_string());
        assert_eq!(value.to_string(), "\"say \\\"hi\\\"\"");
    }

    #[test]
    fn test_display_escape_control_chars() {
        let value = JsonValue::String("\u{01}\t\u{08}".to_string());
        assert_eq!(value.to_string(), "\"\\u0001\\t\\b\"");
    }

    #[test]
    fn test_display_nested() -> Result<()> {
        let mut parser = JsonParser::new(r#"{"arr": [1, 2]}"#)?;
        let value = parser.parse()?;
        let output = value.to_string();
        // Object key order may vary, so check components
        assert!(output.contains("\"arr\""));
        assert!(output.contains("[1,2]"));
        Ok(())
    }

    #[test]
    fn test_pretty_print_escapes_keys_and_values() -> Result<()> {
        let value = JsonValue::Object(HashMap::from([(
            "say \"hi\"".to_string(),
            JsonValue::String("line\nbreak".to_string()),
        )]));

        let pretty = value.pretty_print(2);
        assert!(pretty.contains(r#""say \"hi\"""#));
        assert!(pretty.contains(r#""line\nbreak""#));

        let mut parser = JsonParser::new(&pretty)?;
        assert_eq!(parser.parse()?, value);
        Ok(())
    }

    #[test]
    fn test_pretty_print_string_not_double_quoted() {
        let value = JsonValue::String("hello".to_string());
        assert_eq!(value.pretty_print(2), r#""hello""#);
    }

    #[test]
    fn test_display_nested_array() {
        let value = JsonValue::Array(vec![JsonValue::Array(vec![
            JsonValue::Number(1.0),
            JsonValue::Number(2.0),
        ])]);
        assert_eq!(value.to_string(), "[[1,2]]");
    }
}
