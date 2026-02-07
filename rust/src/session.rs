//! Session management for conversation history.

use parking_lot::Mutex;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Arc;

/// A conversation message.
#[derive(Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
    timestamp: String,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

/// Session metadata stored in JSONL.
#[derive(Serialize, Deserialize)]
struct SessionMetadata {
    #[serde(rename = "_type")]
    type_marker: String,
    created_at: String,
    updated_at: String,
    metadata: HashMap<String, serde_json::Value>,
}

/// A conversation session.
#[pyclass]
pub struct Session {
    #[pyo3(get)]
    key: String,
    messages: Vec<Message>,
    created_at: String,
    updated_at: String,
    metadata: HashMap<String, serde_json::Value>,
}

#[pymethods]
impl Session {
    #[new]
    #[pyo3(signature = (key, messages=None, created_at=None, updated_at=None, metadata=None))]
    fn new(
        key: String,
        messages: Option<&Bound<'_, PyList>>,
        created_at: Option<String>,
        updated_at: Option<String>,
        metadata: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.6f")
            .to_string();

        let msgs = if let Some(py_msgs) = messages {
            let mut result = Vec::new();
            for item in py_msgs.iter() {
                let dict = item.downcast::<PyDict>()?;
                let role: String = dict
                    .get_item("role")?
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("missing 'role'"))?
                    .extract()?;
                let content: String = dict
                    .get_item("content")?
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyKeyError, _>("missing 'content'")
                    })?
                    .extract()?;
                let timestamp: String = dict
                    .get_item("timestamp")?
                    .map(|v| v.extract())
                    .transpose()?
                    .unwrap_or_else(|| now.clone());

                // Collect extra fields
                let mut extra = HashMap::new();
                for (k, v) in dict.iter() {
                    let key: String = k.extract()?;
                    if key != "role" && key != "content" && key != "timestamp" {
                        let value = python_to_json(v)?;
                        extra.insert(key, value);
                    }
                }

                result.push(Message {
                    role,
                    content,
                    timestamp,
                    extra,
                });
            }
            result
        } else {
            Vec::new()
        };

        let meta = if let Some(py_meta) = metadata {
            python_dict_to_json_map(py_meta)?
        } else {
            HashMap::new()
        };

        Ok(Session {
            key,
            messages: msgs,
            created_at: created_at.unwrap_or_else(|| now.clone()),
            updated_at: updated_at.unwrap_or(now),
            metadata: meta,
        })
    }

    /// Add a message to the session.
    #[pyo3(signature = (role, content, **kwargs))]
    fn add_message(
        &mut self,
        _py: Python<'_>,
        role: String,
        content: String,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.6f")
            .to_string();

        let mut extra = HashMap::new();
        if let Some(kw) = kwargs {
            for (k, v) in kw.iter() {
                let key: String = k.extract()?;
                extra.insert(key, python_to_json(v)?);
            }
        }

        self.messages.push(Message {
            role,
            content,
            timestamp: now.clone(),
            extra,
        });
        self.updated_at = now;
        Ok(())
    }

    /// Get message history for LLM context.
    #[pyo3(signature = (max_messages=50))]
    fn get_history(&self, py: Python<'_>, max_messages: usize) -> PyResult<Py<PyList>> {
        let start = if self.messages.len() > max_messages {
            self.messages.len() - max_messages
        } else {
            0
        };

        let result = PyList::empty(py);
        for msg in &self.messages[start..] {
            let dict = PyDict::new(py);
            dict.set_item("role", &msg.role)?;
            dict.set_item("content", &msg.content)?;
            result.append(dict)?;
        }

        Ok(result.into())
    }

    /// Clear all messages in the session.
    fn clear(&mut self) {
        self.messages.clear();
        self.updated_at = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.6f")
            .to_string();
    }

    /// Get created_at timestamp.
    #[getter]
    fn created_at(&self) -> &str {
        &self.created_at
    }

    /// Get updated_at timestamp.
    #[getter]
    fn updated_at(&self) -> &str {
        &self.updated_at
    }

    /// Get messages as Python list.
    #[getter]
    fn messages(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let result = PyList::empty(py);
        for msg in &self.messages {
            let dict = PyDict::new(py);
            dict.set_item("role", &msg.role)?;
            dict.set_item("content", &msg.content)?;
            dict.set_item("timestamp", &msg.timestamp)?;
            for (k, v) in &msg.extra {
                dict.set_item(k, json_to_python(py, v)?)?;
            }
            result.append(dict)?;
        }
        Ok(result.into())
    }

    /// Get metadata as Python dict.
    #[getter]
    fn metadata(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.metadata {
            dict.set_item(k, json_to_python(py, v)?)?;
        }
        Ok(dict.into())
    }

    /// Set metadata from Python dict.
    #[setter]
    fn set_metadata(&mut self, value: &Bound<'_, PyDict>) -> PyResult<()> {
        self.metadata = python_dict_to_json_map(value)?;
        Ok(())
    }
}

/// Manages conversation sessions.
#[pyclass]
#[allow(dead_code)]
pub struct SessionManager {
    workspace: PathBuf,
    sessions_dir: PathBuf,
    cache: Arc<Mutex<HashMap<String, SessionData>>>,
}

/// Internal session data for caching.
struct SessionData {
    key: String,
    messages: Vec<Message>,
    created_at: String,
    updated_at: String,
    metadata: HashMap<String, serde_json::Value>,
}

impl SessionData {
    fn to_session(&self) -> Session {
        Session {
            key: self.key.clone(),
            messages: self.messages.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            metadata: self.metadata.clone(),
        }
    }

    fn from_session(session: &Session) -> Self {
        SessionData {
            key: session.key.clone(),
            messages: session.messages.clone(),
            created_at: session.created_at.clone(),
            updated_at: session.updated_at.clone(),
            metadata: session.metadata.clone(),
        }
    }
}

#[pymethods]
impl SessionManager {
    #[new]
    fn new(workspace: PathBuf) -> PyResult<Self> {
        let sessions_dir = dirs::home_dir()
            .ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Cannot find home directory")
            })?
            .join(".debot")
            .join("sessions");

        // Ensure directory exists
        fs::create_dir_all(&sessions_dir).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to create sessions directory: {}",
                e
            ))
        })?;

        Ok(SessionManager {
            workspace,
            sessions_dir,
            cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get an existing session or create a new one.
    fn get_or_create(&self, key: String) -> PyResult<Session> {
        // Check cache first
        {
            let cache = self.cache.lock();
            if let Some(data) = cache.get(&key) {
                return Ok(data.to_session());
            }
        }

        // Try to load from disk
        let session = match self.load(&key) {
            Ok(Some(s)) => s,
            Ok(None) => {
                let now = chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S%.6f")
                    .to_string();
                Session {
                    key: key.clone(),
                    messages: Vec::new(),
                    created_at: now.clone(),
                    updated_at: now,
                    metadata: HashMap::new(),
                }
            }
            Err(_e) => {
                // Log warning and create new session
                let now = chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S%.6f")
                    .to_string();
                Session {
                    key: key.clone(),
                    messages: Vec::new(),
                    created_at: now.clone(),
                    updated_at: now,
                    metadata: HashMap::new(),
                }
            }
        };

        // Cache it
        {
            let mut cache = self.cache.lock();
            cache.insert(key, SessionData::from_session(&session));
        }

        Ok(session)
    }

    /// Save a session to disk.
    fn save(&self, session: &Session) -> PyResult<()> {
        let path = self.get_session_path(&session.key);

        let mut file = File::create(&path).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to create session file: {}",
                e
            ))
        })?;

        // Write metadata first
        let metadata = SessionMetadata {
            type_marker: "metadata".to_string(),
            created_at: session.created_at.clone(),
            updated_at: session.updated_at.clone(),
            metadata: session.metadata.clone(),
        };
        let meta_json = serde_json::to_string(&metadata).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to serialize metadata: {}",
                e
            ))
        })?;
        writeln!(file, "{}", meta_json).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to write metadata: {}", e))
        })?;

        // Write messages
        for msg in &session.messages {
            let msg_json = serde_json::to_string(msg).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Failed to serialize message: {}",
                    e
                ))
            })?;
            writeln!(file, "{}", msg_json).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                    "Failed to write message: {}",
                    e
                ))
            })?;
        }

        // Update cache
        {
            let mut cache = self.cache.lock();
            cache.insert(session.key.clone(), SessionData::from_session(session));
        }

        Ok(())
    }

    /// Delete a session.
    fn delete(&self, key: String) -> PyResult<bool> {
        // Remove from cache
        {
            let mut cache = self.cache.lock();
            cache.remove(&key);
        }

        // Remove file
        let path = self.get_session_path(&key);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                    "Failed to delete session: {}",
                    e
                ))
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all sessions.
    fn list_sessions(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let result = PyList::empty(py);

        let entries = fs::read_dir(&self.sessions_dir).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to read sessions directory: {}",
                e
            ))
        })?;

        let mut sessions: Vec<(String, String, String, String)> = Vec::new();

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            // Read just the first line (metadata)
            let file = match File::open(&path) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let reader = BufReader::new(file);
            let first_line = match reader.lines().next() {
                Some(Ok(line)) => line,
                _ => continue,
            };

            let data: serde_json::Value = match serde_json::from_str(&first_line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if data.get("_type").and_then(|v| v.as_str()) != Some("metadata") {
                continue;
            }

            let key = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.replace("_", ":"))
                .unwrap_or_default();
            let created_at = data
                .get("created_at")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let updated_at = data
                .get("updated_at")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let path_str = path.to_string_lossy().to_string();

            sessions.push((key, created_at, updated_at, path_str));
        }

        // Sort by updated_at descending
        sessions.sort_by(|a, b| b.2.cmp(&a.2));

        for (key, created_at, updated_at, path_str) in sessions {
            let dict = PyDict::new(py);
            dict.set_item("key", key)?;
            dict.set_item("created_at", created_at)?;
            dict.set_item("updated_at", updated_at)?;
            dict.set_item("path", path_str)?;
            result.append(dict)?;
        }

        Ok(result.into())
    }
}

impl SessionManager {
    fn get_session_path(&self, key: &str) -> PathBuf {
        let safe_key = safe_filename(&key.replace(":", "_"));
        self.sessions_dir.join(format!("{}.jsonl", safe_key))
    }

    fn load(&self, key: &str) -> Result<Option<Session>, String> {
        let path = self.get_session_path(key);

        if !path.exists() {
            return Ok(None);
        }

        let file = File::open(&path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);

        let mut messages = Vec::new();
        let mut metadata = HashMap::new();
        let mut created_at = None;

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let data: serde_json::Value = serde_json::from_str(line).map_err(|e| e.to_string())?;

            if data.get("_type").and_then(|v| v.as_str()) == Some("metadata") {
                if let Some(meta) = data.get("metadata") {
                    if let Some(obj) = meta.as_object() {
                        for (k, v) in obj {
                            metadata.insert(k.clone(), v.clone());
                        }
                    }
                }
                created_at = data
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .map(String::from);
            } else {
                let msg: Message = serde_json::from_value(data).map_err(|e| e.to_string())?;
                messages.push(msg);
            }
        }

        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.6f")
            .to_string();

        Ok(Some(Session {
            key: key.to_string(),
            messages,
            created_at: created_at.unwrap_or_else(|| now.clone()),
            updated_at: now,
            metadata,
        }))
    }
}

/// Convert a string to a safe filename.
fn safe_filename(name: &str) -> String {
    let unsafe_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let mut result = name.to_string();
    for c in unsafe_chars {
        result = result.replace(c, "_");
    }
    result.trim().to_string()
}

/// Convert Python object to JSON value.
fn python_to_json(obj: Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    if obj.is_none() {
        Ok(serde_json::Value::Null)
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(serde_json::Value::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(serde_json::Value::Number(i.into()))
    } else if let Ok(f) = obj.extract::<f64>() {
        Ok(serde_json::json!(f))
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(serde_json::Value::String(s))
    } else if let Ok(list) = obj.downcast::<PyList>() {
        let mut arr = Vec::new();
        for item in list.iter() {
            arr.push(python_to_json(item)?);
        }
        Ok(serde_json::Value::Array(arr))
    } else if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            map.insert(key, python_to_json(v)?);
        }
        Ok(serde_json::Value::Object(map))
    } else {
        // Fallback: convert to string
        Ok(serde_json::Value::String(obj.str()?.to_string()))
    }
}

/// Convert Python dict to HashMap<String, serde_json::Value>.
fn python_dict_to_json_map(
    dict: &Bound<'_, PyDict>,
) -> PyResult<HashMap<String, serde_json::Value>> {
    let mut map = HashMap::new();
    for (k, v) in dict.iter() {
        let key: String = k.extract()?;
        map.insert(key, python_to_json(v)?);
    }
    Ok(map)
}

/// Convert JSON value to Python object.
fn json_to_python(py: Python<'_>, value: &serde_json::Value) -> PyResult<PyObject> {
    use pyo3::types::PyBool;

    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => {
            let py_bool = PyBool::new(py, *b);
            Ok(py_bool.to_owned().into_any().unbind())
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any().unbind())
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_pyobject(py)?.into_any().unbind())
            } else {
                Ok(py.None())
            }
        }
        serde_json::Value::String(s) => Ok(PyString::new(py, s).into_any().unbind()),
        serde_json::Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_to_python(py, item)?)?;
            }
            Ok(list.into())
        }
        serde_json::Value::Object(obj) => {
            let dict = PyDict::new(py);
            for (k, v) in obj {
                dict.set_item(k, json_to_python(py, v)?)?;
            }
            Ok(dict.into())
        }
    }
}
