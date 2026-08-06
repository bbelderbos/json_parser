use crate::{JsonError, JsonValue, parse};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;

impl<'py> IntoPyObject<'py> for JsonValue {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        match self {
            JsonValue::Null => Ok(py.None().into_bound(py)),
            JsonValue::Boolean(b) => Ok(b.into_pyobject(py)?.to_owned().into_any()),
            JsonValue::Number(n) => Ok(n.into_pyobject(py)?.to_owned().into_any()),
            JsonValue::String(s) => Ok(s.into_pyobject(py)?.into_any()),
            JsonValue::Array(arr) => {
                let py_list = PyList::empty(py);
                for item in arr {
                    py_list.append(item.into_pyobject(py)?)?;
                }
                Ok(py_list.into_any())
            }
            JsonValue::Object(obj) => {
                let py_dict = PyDict::new(py);
                for (k, v) in obj {
                    py_dict.set_item(k, v.into_pyobject(py)?)?;
                }
                Ok(py_dict.into_any())
            }
        }
    }
}

impl From<JsonError> for PyErr {
    fn from(err: JsonError) -> PyErr {
        match err {
            JsonError::UnexpectedToken {
                expected,
                found,
                position,
            } => PyValueError::new_err(format!(
                "Unexpected token at position {}: expected {}, found {}",
                position, expected, found
            )),
            JsonError::UnexpectedEndOfInput { expected, position } => {
                PyValueError::new_err(format!(
                    "Unexpected end of input at position {}: expected {}",
                    position, expected
                ))
            }
            JsonError::InvalidNumber { value, position } => PyValueError::new_err(format!(
                "Invalid number '{}' at position {}",
                value, position
            )),
            JsonError::UnterminatedString { position } => PyValueError::new_err(format!(
                "Unterminated string starting at position {}",
                position
            )),
            JsonError::InvalidEscape { char, position } => PyValueError::new_err(format!(
                "Invalid escape character '{}' at position {}",
                char, position
            )),
            JsonError::InvalidUnicode { sequence, position } => PyValueError::new_err(format!(
                "Invalid unicode sequence '{}' at position {}",
                sequence, position
            )),
        }
    }
}

/// Parse a JSON string into Python objects.
///
/// # Example
///
/// ```python
/// >>> from rust_json_parser import parse_json
/// >>> parse_json('{"name": "Alice"}')
/// {'name': 'Alice'}
/// >>> parse_json('[1, true, null]')
/// [1.0, True, None]
/// ```
///
/// # Errors
///
/// Raises `ValueError` for any malformed input — a stray token, an unclosed array,
/// object or string, an unparseable number, or a bad escape sequence. The message
/// carries the position in the input where parsing failed.
#[pyfunction]
fn parse_json<'py>(py: Python<'py>, input: &str) -> PyResult<Bound<'py, PyAny>> {
    parse(input)?.into_pyobject(py)
}

/// Read a file and parse its contents into Python objects.
///
/// # Example
///
/// ```python
/// >>> from rust_json_parser import parse_json_file
/// >>> parse_json_file("config.json")
/// {'debug': True}
/// ```
///
/// # Errors
///
/// Raises `FileNotFoundError` for a missing path, `PermissionError` if it cannot be
/// opened, and plain `OSError` for anything else that blocks the read, including
/// contents that are not valid UTF-8. Raises `ValueError` on malformed JSON, same as
/// [`parse_json`].
#[pyfunction]
fn parse_json_file<'py>(py: Python<'py>, file_path: &str) -> PyResult<Bound<'py, PyAny>> {
    let input = std::fs::read_to_string(file_path)?;
    parse(&input)?.into_pyobject(py)
}

#[pyfunction]
#[pyo3(signature = (obj, indent=None))]
fn dumps(obj: &Bound<PyAny>, indent: Option<usize>) -> PyResult<String> {
    let json_value = py_to_json_value(obj)?;
    let json_string = match indent {
        Some(i) => json_value.pretty_print(i),
        None => json_value.to_string(),
    };
    Ok(json_string)
}

fn py_to_json_value(obj: &Bound<PyAny>) -> PyResult<JsonValue> {
    if obj.is_none() {
        return Ok(JsonValue::Null);
    }

    // needs to be checked before number because of Python inheritance (bool is a subclass of int)
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(JsonValue::Boolean(b));
    }

    if let Ok(n) = obj.extract::<f64>() {
        return Ok(JsonValue::Number(n));
    }

    if let Ok(s) = obj.extract::<String>() {
        return Ok(JsonValue::String(s));
    }

    if let Ok(list) = obj.cast::<PyList>() {
        let mut arr = Vec::new();
        for item in list.iter() {
            arr.push(py_to_json_value(&item)?);
        }
        return Ok(JsonValue::Array(arr));
    }

    if let Ok(dict) = obj.cast::<PyDict>() {
        let mut map = HashMap::new();
        for (key, value) in dict.iter() {
            let key_str = key.extract::<String>()?;
            let value_json = py_to_json_value(&value)?;
            map.insert(key_str, value_json);
        }
        return Ok(JsonValue::Object(map));
    }

    // fallthrough of unsupported types
    Err(PyValueError::new_err(
        "Unsupported type for JSON conversion",
    ))
}

#[pymodule]
fn _rust_json_parser(m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_json, m)?)?;
    m.add_function(wrap_pyfunction!(parse_json_file, m)?)?;
    m.add_function(wrap_pyfunction!(dumps, m)?)?;
    Ok(())
}
