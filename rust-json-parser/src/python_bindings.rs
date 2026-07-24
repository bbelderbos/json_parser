use crate::{parse, JsonError, JsonValue};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

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

#[pyfunction]
fn parse_json<'py>(py: Python<'py>, input: &str) -> PyResult<Bound<'py, PyAny>> {
    parse(input)?.into_pyobject(py)
}

#[pyfunction]
fn parse_json_file<'py>(py: Python<'py>, file_path: &str) -> PyResult<Bound<'py, PyAny>> {
    let input = std::fs::read_to_string(file_path)?;
    parse(&input)?.into_pyobject(py)
}

#[pymodule]
fn _rust_json_parser(m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_json, m)?)?;
    m.add_function(wrap_pyfunction!(parse_json_file, m)?)?;
    //m.add_function(wrap_pyfunction!(dumps, m)?)?;
    Ok(())
}
