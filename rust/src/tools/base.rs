//! Base tool trait and types.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::sync::Arc;

/// Wrapper for PyObject to make it Clone-able.
#[derive(Clone)]
pub struct ClonablePyObject(Arc<PyObject>);

impl ClonablePyObject {
    pub fn new(obj: PyObject) -> Self {
        ClonablePyObject(Arc::new(obj))
    }

    pub fn get(&self) -> &PyObject {
        &self.0
    }
}

/// Tool schema in OpenAI function format.
#[pyclass]
#[derive(Clone)]
pub struct ToolSchema {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub description: String,
    pub parameters: ClonablePyObject,
}

#[pymethods]
impl ToolSchema {
    #[getter]
    fn get_parameters(&self, py: Python<'_>) -> PyObject {
        self.parameters.get().clone_ref(py)
    }

    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        let func = PyDict::new(py);

        func.set_item("name", &self.name)?;
        func.set_item("description", &self.description)?;
        func.set_item("parameters", self.parameters.get().bind(py))?;

        dict.set_item("type", "function")?;
        dict.set_item("function", func)?;

        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("ToolSchema(name={:?})", self.name)
    }
}

/// Trait for tools - implemented by each concrete tool type.
///
/// In PyO3, we can't use Rust traits directly with Python, so we use
/// a common interface pattern where each tool implements these methods.
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> HashMap<String, serde_json::Value>;

    fn to_schema(&self, py: Python<'_>) -> PyResult<ToolSchema> {
        let params = serde_json::to_string(&self.parameters())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let params_obj = py
            .import("json")?
            .call_method1("loads", (params,))?
            .unbind();

        Ok(ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: ClonablePyObject::new(params_obj),
        })
    }
}

/// Helper to create a standard JSON Schema object type.
pub fn object_schema(
    properties: HashMap<String, serde_json::Value>,
    required: Vec<&str>,
) -> HashMap<String, serde_json::Value> {
    let mut schema = HashMap::new();
    schema.insert("type".into(), serde_json::json!("object"));
    schema.insert("properties".into(), serde_json::json!(properties));
    schema.insert("required".into(), serde_json::json!(required));
    schema
}

/// Helper to create a string property schema.
pub fn string_prop(description: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "string",
        "description": description
    })
}

/// Helper to create an integer property schema.
#[allow(dead_code)]
pub fn int_prop(description: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "integer",
        "description": description
    })
}
