//! Shell execution tool.

use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use super::base::{object_schema, string_prop, Tool, ToolSchema};

/// Tool to execute shell commands.
#[pyclass]
#[derive(Clone)]
pub struct ExecTool {
    timeout_secs: u64,
    working_dir: Option<String>,
}

impl Tool for ExecTool {
    fn name(&self) -> &str {
        "exec"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return its output. Use with caution."
    }

    fn parameters(&self) -> HashMap<String, serde_json::Value> {
        let mut props = HashMap::new();
        props.insert(
            "command".into(),
            string_prop("The shell command to execute"),
        );
        props.insert(
            "working_dir".into(),
            string_prop("Optional working directory for the command"),
        );
        object_schema(props, vec!["command"])
    }
}

impl ExecTool {
    pub fn tool_name(&self) -> &str {
        "exec"
    }

    pub fn to_schema(&self, py: Python<'_>) -> PyResult<ToolSchema> {
        Tool::to_schema(self, py)
    }

    pub async fn execute_inner(&self, params: &HashMap<String, String>) -> String {
        let command = match params.get("command") {
            Some(c) => c,
            None => return "Error: Missing required parameter 'command'".to_string(),
        };

        let cwd = params
            .get("working_dir")
            .map(|s| s.as_str())
            .or(self.working_dir.as_deref())
            .map(|s| {
                if let Some(stripped) = s.strip_prefix("~/") {
                    if let Some(home) = dirs::home_dir() {
                        return home.join(stripped);
                    }
                }
                PathBuf::from(s)
            })
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Create shell command
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", command]);
            c
        };

        cmd.current_dir(&cwd);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        // Execute with timeout
        let result = timeout(Duration::from_secs(self.timeout_secs), async {
            match cmd.output().await {
                Ok(output) => {
                    let mut parts = Vec::new();

                    // stdout
                    if !output.stdout.is_empty() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        parts.push(stdout.to_string());
                    }

                    // stderr
                    if !output.stderr.is_empty() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        if !stderr.trim().is_empty() {
                            parts.push(format!("STDERR:\n{}", stderr));
                        }
                    }

                    // Exit code if non-zero
                    if !output.status.success() {
                        let code = output.status.code().unwrap_or(-1);
                        parts.push(format!("\nExit code: {}", code));
                    }

                    let result = if parts.is_empty() {
                        "(no output)".to_string()
                    } else {
                        parts.join("\n")
                    };

                    // Truncate very long output
                    const MAX_LEN: usize = 10000;
                    if result.len() > MAX_LEN {
                        format!(
                            "{}... (truncated, {} more chars)",
                            &result[..MAX_LEN],
                            result.len() - MAX_LEN
                        )
                    } else {
                        result
                    }
                }
                Err(e) => format!("Error executing command: {}", e),
            }
        })
        .await;

        match result {
            Ok(output) => output,
            Err(_) => format!(
                "Error: Command timed out after {} seconds",
                self.timeout_secs
            ),
        }
    }
}

#[pymethods]
impl ExecTool {
    #[new]
    #[pyo3(signature = (timeout=60, working_dir=None))]
    fn new(timeout: u64, working_dir: Option<String>) -> Self {
        Self {
            timeout_secs: timeout,
            working_dir,
        }
    }

    #[getter]
    fn name(&self) -> &str {
        "exec"
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

    #[pyo3(signature = (command, working_dir=None))]
    fn execute<'py>(
        &self,
        py: Python<'py>,
        command: String,
        working_dir: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let this = self.clone();
        future_into_py(py, async move {
            let mut params = HashMap::new();
            params.insert("command".to_string(), command);
            if let Some(wd) = working_dir {
                params.insert("working_dir".to_string(), wd);
            }
            Ok(this.execute_inner(&params).await)
        })
    }

    fn to_schema_py(&self, py: Python<'_>) -> PyResult<PyObject> {
        let schema = Tool::to_schema(self, py)?;
        schema.to_dict(py)
    }
}
