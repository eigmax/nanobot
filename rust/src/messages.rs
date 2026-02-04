use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Thread-safe wrapper for Python objects in metadata.
#[derive(Clone)]
struct PyValue(Arc<PyObject>);

impl PyValue {
    fn new(obj: PyObject) -> Self {
        PyValue(Arc::new(obj))
    }

    fn get(&self) -> &PyObject {
        &self.0
    }
}

/// Message received from a chat channel.
#[pyclass]
#[derive(Clone)]
pub struct InboundMessage {
    #[pyo3(get, set)]
    pub channel: String,
    #[pyo3(get, set)]
    pub sender_id: String,
    #[pyo3(get, set)]
    pub chat_id: String,
    #[pyo3(get, set)]
    pub content: String,
    #[pyo3(get, set)]
    pub timestamp: f64,
    #[pyo3(get, set)]
    pub media: Vec<String>,
    metadata: HashMap<String, PyValue>,
}

#[pymethods]
impl InboundMessage {
    #[new]
    #[pyo3(signature = (channel, sender_id, chat_id, content, timestamp=None, media=None, metadata=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        _py: Python<'_>,
        channel: String,
        sender_id: String,
        chat_id: String,
        content: String,
        timestamp: Option<f64>,
        media: Option<Vec<String>>,
        metadata: Option<Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let ts = timestamp.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0)
        });

        let meta = if let Some(dict) = metadata {
            let mut map = HashMap::new();
            for (key, value) in dict.iter() {
                let key_str: String = key.extract()?;
                map.insert(key_str, PyValue::new(value.unbind()));
            }
            map
        } else {
            HashMap::new()
        };

        Ok(Self {
            channel,
            sender_id,
            chat_id,
            content,
            timestamp: ts,
            media: media.unwrap_or_default(),
            metadata: meta,
        })
    }

    /// Unique key for session identification.
    #[getter]
    fn session_key(&self) -> String {
        format!("{}:{}", self.channel, self.chat_id)
    }

    #[getter]
    fn metadata(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (key, value) in &self.metadata {
            dict.set_item(key, value.get().bind(py))?;
        }
        Ok(dict.into())
    }

    #[setter]
    fn set_metadata(&mut self, _py: Python<'_>, value: Bound<'_, PyDict>) -> PyResult<()> {
        let mut map = HashMap::new();
        for (key, val) in value.iter() {
            let key_str: String = key.extract()?;
            map.insert(key_str, PyValue::new(val.unbind()));
        }
        self.metadata = map;
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!(
            "InboundMessage(channel={:?}, sender_id={:?}, chat_id={:?}, content={:?})",
            self.channel, self.sender_id, self.chat_id, self.content
        )
    }
}

/// Message to send to a chat channel.
#[pyclass]
#[derive(Clone)]
pub struct OutboundMessage {
    #[pyo3(get, set)]
    pub channel: String,
    #[pyo3(get, set)]
    pub chat_id: String,
    #[pyo3(get, set)]
    pub content: String,
    #[pyo3(get, set)]
    pub reply_to: Option<String>,
    #[pyo3(get, set)]
    pub media: Vec<String>,
    metadata: HashMap<String, PyValue>,
}

#[pymethods]
impl OutboundMessage {
    #[new]
    #[pyo3(signature = (channel, chat_id, content, reply_to=None, media=None, metadata=None))]
    fn new(
        _py: Python<'_>,
        channel: String,
        chat_id: String,
        content: String,
        reply_to: Option<String>,
        media: Option<Vec<String>>,
        metadata: Option<Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let meta = if let Some(dict) = metadata {
            let mut map = HashMap::new();
            for (key, value) in dict.iter() {
                let key_str: String = key.extract()?;
                map.insert(key_str, PyValue::new(value.unbind()));
            }
            map
        } else {
            HashMap::new()
        };

        Ok(Self {
            channel,
            chat_id,
            content,
            reply_to,
            media: media.unwrap_or_default(),
            metadata: meta,
        })
    }

    #[getter]
    fn metadata(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (key, value) in &self.metadata {
            dict.set_item(key, value.get().bind(py))?;
        }
        Ok(dict.into())
    }

    #[setter]
    fn set_metadata(&mut self, _py: Python<'_>, value: Bound<'_, PyDict>) -> PyResult<()> {
        let mut map = HashMap::new();
        for (key, val) in value.iter() {
            let key_str: String = key.extract()?;
            map.insert(key_str, PyValue::new(val.unbind()));
        }
        self.metadata = map;
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!(
            "OutboundMessage(channel={:?}, chat_id={:?}, content={:?})",
            self.channel, self.chat_id, self.content
        )
    }
}
