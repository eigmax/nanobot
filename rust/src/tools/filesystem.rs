//! File system tools: read, write, edit, list directory.

use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

use super::base::{object_schema, string_prop, Tool, ToolSchema};

/// Expand ~ to home directory.
fn expand_path(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(path)
}

// ============================================================================
// ReadFileTool
// ============================================================================

/// Tool to read file contents.
#[pyclass]
#[derive(Clone)]
pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file at the given path."
    }

    fn parameters(&self) -> HashMap<String, serde_json::Value> {
        let mut props = HashMap::new();
        props.insert("path".into(), string_prop("The file path to read"));
        object_schema(props, vec!["path"])
    }
}

impl ReadFileTool {
    pub fn tool_name(&self) -> &str {
        "read_file"
    }

    pub fn to_schema(&self, py: Python<'_>) -> PyResult<ToolSchema> {
        Tool::to_schema(self, py)
    }

    pub async fn execute_inner(&self, params: &HashMap<String, String>) -> String {
        let path = match params.get("path") {
            Some(p) => p,
            None => return "Error: Missing required parameter 'path'".to_string(),
        };

        let file_path = expand_path(path);

        match fs::read_to_string(&file_path).await {
            Ok(content) => content,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    format!("Error: File not found: {}", path)
                } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                    format!("Error: Permission denied: {}", path)
                } else {
                    format!("Error reading file: {}", e)
                }
            }
        }
    }
}

#[pymethods]
impl ReadFileTool {
    #[new]
    fn new() -> Self {
        Self
    }

    #[getter]
    fn name(&self) -> &str {
        "read_file"
    }

    #[getter]
    fn description(&self) -> &str {
        Tool::description(self)
    }

    #[getter]
    fn parameters(&self, py: Python<'_>) -> PyResult<PyObject> {
        let params = Tool::parameters(self);
        let json_str = serde_json::to_string(&params)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let result = py.import("json")?.call_method1("loads", (json_str,))?;
        Ok(result.into())
    }

    fn execute<'py>(&self, py: Python<'py>, path: String) -> PyResult<Bound<'py, PyAny>> {
        let this = self.clone();
        future_into_py(py, async move {
            let mut params = HashMap::new();
            params.insert("path".to_string(), path);
            Ok(this.execute_inner(&params).await)
        })
    }

    fn to_schema_py(&self, py: Python<'_>) -> PyResult<PyObject> {
        let schema = Tool::to_schema(self, py)?;
        schema.to_dict(py)
    }
}

// ============================================================================
// WriteFileTool
// ============================================================================

/// Tool to write content to a file.
#[pyclass]
#[derive(Clone)]
pub struct WriteFileTool;

impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file at the given path. Creates parent directories if needed."
    }

    fn parameters(&self) -> HashMap<String, serde_json::Value> {
        let mut props = HashMap::new();
        props.insert("path".into(), string_prop("The file path to write to"));
        props.insert("content".into(), string_prop("The content to write"));
        object_schema(props, vec!["path", "content"])
    }
}

impl WriteFileTool {
    pub fn tool_name(&self) -> &str {
        "write_file"
    }

    pub fn to_schema(&self, py: Python<'_>) -> PyResult<ToolSchema> {
        Tool::to_schema(self, py)
    }

    pub async fn execute_inner(&self, params: &HashMap<String, String>) -> String {
        let path = match params.get("path") {
            Some(p) => p,
            None => return "Error: Missing required parameter 'path'".to_string(),
        };
        let content = match params.get("content") {
            Some(c) => c,
            None => return "Error: Missing required parameter 'content'".to_string(),
        };

        let file_path = expand_path(path);

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            if let Err(e) = fs::create_dir_all(parent).await {
                return format!("Error creating directories: {}", e);
            }
        }

        match fs::write(&file_path, content).await {
            Ok(()) => format!("Successfully wrote {} bytes to {}", content.len(), path),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    format!("Error: Permission denied: {}", path)
                } else {
                    format!("Error writing file: {}", e)
                }
            }
        }
    }
}

#[pymethods]
impl WriteFileTool {
    #[new]
    fn new() -> Self {
        Self
    }

    #[getter]
    fn name(&self) -> &str {
        "write_file"
    }

    #[getter]
    fn description(&self) -> &str {
        Tool::description(self)
    }

    #[getter]
    fn parameters(&self, py: Python<'_>) -> PyResult<PyObject> {
        let params = Tool::parameters(self);
        let json_str = serde_json::to_string(&params)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let result = py.import("json")?.call_method1("loads", (json_str,))?;
        Ok(result.into())
    }

    fn execute<'py>(
        &self,
        py: Python<'py>,
        path: String,
        content: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let this = self.clone();
        future_into_py(py, async move {
            let mut params = HashMap::new();
            params.insert("path".to_string(), path);
            params.insert("content".to_string(), content);
            Ok(this.execute_inner(&params).await)
        })
    }

    fn to_schema_py(&self, py: Python<'_>) -> PyResult<PyObject> {
        let schema = Tool::to_schema(self, py)?;
        schema.to_dict(py)
    }
}

// ============================================================================
// EditFileTool
// ============================================================================

/// Tool to edit a file by replacing text.
#[pyclass]
#[derive(Clone)]
pub struct EditFileTool;

impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing old_text with new_text. The old_text must exist exactly in the file."
    }

    fn parameters(&self) -> HashMap<String, serde_json::Value> {
        let mut props = HashMap::new();
        props.insert("path".into(), string_prop("The file path to edit"));
        props.insert(
            "old_text".into(),
            string_prop("The exact text to find and replace"),
        );
        props.insert("new_text".into(), string_prop("The text to replace with"));
        object_schema(props, vec!["path", "old_text", "new_text"])
    }
}

impl EditFileTool {
    pub fn tool_name(&self) -> &str {
        "edit_file"
    }

    pub fn to_schema(&self, py: Python<'_>) -> PyResult<ToolSchema> {
        Tool::to_schema(self, py)
    }

    pub async fn execute_inner(&self, params: &HashMap<String, String>) -> String {
        let path = match params.get("path") {
            Some(p) => p,
            None => return "Error: Missing required parameter 'path'".to_string(),
        };
        let old_text = match params.get("old_text") {
            Some(t) => t,
            None => return "Error: Missing required parameter 'old_text'".to_string(),
        };
        let new_text = match params.get("new_text") {
            Some(t) => t,
            None => return "Error: Missing required parameter 'new_text'".to_string(),
        };

        let file_path = expand_path(path);

        // Read current content
        let content = match fs::read_to_string(&file_path).await {
            Ok(c) => c,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    return format!("Error: File not found: {}", path);
                }
                return format!("Error reading file: {}", e);
            }
        };

        // Check if old_text exists
        if !content.contains(old_text) {
            return "Error: old_text not found in file. Make sure it matches exactly.".to_string();
        }

        // Count occurrences
        let count = content.matches(old_text).count();
        if count > 1 {
            return format!(
                "Warning: old_text appears {} times. Please provide more context to make it unique.",
                count
            );
        }

        // Replace and write
        let new_content = content.replacen(old_text, new_text, 1);

        match fs::write(&file_path, new_content).await {
            Ok(()) => format!("Successfully edited {}", path),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    format!("Error: Permission denied: {}", path)
                } else {
                    format!("Error writing file: {}", e)
                }
            }
        }
    }
}

#[pymethods]
impl EditFileTool {
    #[new]
    fn new() -> Self {
        Self
    }

    #[getter]
    fn name(&self) -> &str {
        "edit_file"
    }

    #[getter]
    fn description(&self) -> &str {
        Tool::description(self)
    }

    #[getter]
    fn parameters(&self, py: Python<'_>) -> PyResult<PyObject> {
        let params = Tool::parameters(self);
        let json_str = serde_json::to_string(&params)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let result = py.import("json")?.call_method1("loads", (json_str,))?;
        Ok(result.into())
    }

    fn execute<'py>(
        &self,
        py: Python<'py>,
        path: String,
        old_text: String,
        new_text: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let this = self.clone();
        future_into_py(py, async move {
            let mut params = HashMap::new();
            params.insert("path".to_string(), path);
            params.insert("old_text".to_string(), old_text);
            params.insert("new_text".to_string(), new_text);
            Ok(this.execute_inner(&params).await)
        })
    }

    fn to_schema_py(&self, py: Python<'_>) -> PyResult<PyObject> {
        let schema = Tool::to_schema(self, py)?;
        schema.to_dict(py)
    }
}

// ============================================================================
// ListDirTool
// ============================================================================

/// Tool to list directory contents.
#[pyclass]
#[derive(Clone)]
pub struct ListDirTool;

impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "List the contents of a directory."
    }

    fn parameters(&self) -> HashMap<String, serde_json::Value> {
        let mut props = HashMap::new();
        props.insert("path".into(), string_prop("The directory path to list"));
        object_schema(props, vec!["path"])
    }
}

impl ListDirTool {
    pub fn tool_name(&self) -> &str {
        "list_dir"
    }

    pub fn to_schema(&self, py: Python<'_>) -> PyResult<ToolSchema> {
        Tool::to_schema(self, py)
    }

    pub async fn execute_inner(&self, params: &HashMap<String, String>) -> String {
        let path = match params.get("path") {
            Some(p) => p,
            None => return "Error: Missing required parameter 'path'".to_string(),
        };

        let dir_path = expand_path(path);

        // Check if path exists and is a directory
        let metadata = match fs::metadata(&dir_path).await {
            Ok(m) => m,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    return format!("Error: Directory not found: {}", path);
                }
                return format!("Error: {}", e);
            }
        };

        if !metadata.is_dir() {
            return format!("Error: Not a directory: {}", path);
        }

        // Read directory entries
        let mut entries = match fs::read_dir(&dir_path).await {
            Ok(e) => e,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    return format!("Error: Permission denied: {}", path);
                }
                return format!("Error listing directory: {}", e);
            }
        };

        let mut items: Vec<(String, bool)> = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
            items.push((name, is_dir));
        }

        if items.is_empty() {
            return format!("Directory {} is empty", path);
        }

        // Sort items
        items.sort_by(|a, b| a.0.cmp(&b.0));

        // Format output
        let output: Vec<String> = items
            .into_iter()
            .map(|(name, is_dir)| {
                let prefix = if is_dir { "\u{1F4C1} " } else { "\u{1F4C4} " };
                format!("{}{}", prefix, name)
            })
            .collect();

        output.join("\n")
    }
}

#[pymethods]
impl ListDirTool {
    #[new]
    fn new() -> Self {
        Self
    }

    #[getter]
    fn name(&self) -> &str {
        "list_dir"
    }

    #[getter]
    fn description(&self) -> &str {
        Tool::description(self)
    }

    #[getter]
    fn parameters(&self, py: Python<'_>) -> PyResult<PyObject> {
        let params = Tool::parameters(self);
        let json_str = serde_json::to_string(&params)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let result = py.import("json")?.call_method1("loads", (json_str,))?;
        Ok(result.into())
    }

    fn execute<'py>(&self, py: Python<'py>, path: String) -> PyResult<Bound<'py, PyAny>> {
        let this = self.clone();
        future_into_py(py, async move {
            let mut params = HashMap::new();
            params.insert("path".to_string(), path);
            Ok(this.execute_inner(&params).await)
        })
    }

    fn to_schema_py(&self, py: Python<'_>) -> PyResult<PyObject> {
        let schema = Tool::to_schema(self, py)?;
        schema.to_dict(py)
    }
}
