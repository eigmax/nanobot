//! Tool registry for managing and executing tools.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_async_runtimes::tokio::future_into_py;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::base::ToolSchema;
use super::filesystem::{EditFileTool, ListDirTool, ReadFileTool, WriteFileTool};
use super::shell::ExecTool;

/// Internal enum to hold different tool types.
#[derive(Clone)]
enum ToolType {
    ReadFile(ReadFileTool),
    WriteFile(WriteFileTool),
    EditFile(EditFileTool),
    ListDir(ListDirTool),
    Exec(ExecTool),
}

impl ToolType {
    #[allow(dead_code)]
    fn name(&self) -> &str {
        match self {
            ToolType::ReadFile(t) => t.tool_name(),
            ToolType::WriteFile(t) => t.tool_name(),
            ToolType::EditFile(t) => t.tool_name(),
            ToolType::ListDir(t) => t.tool_name(),
            ToolType::Exec(t) => t.tool_name(),
        }
    }

    fn to_schema(&self, py: Python<'_>) -> PyResult<ToolSchema> {
        match self {
            ToolType::ReadFile(t) => t.to_schema(py),
            ToolType::WriteFile(t) => t.to_schema(py),
            ToolType::EditFile(t) => t.to_schema(py),
            ToolType::ListDir(t) => t.to_schema(py),
            ToolType::Exec(t) => t.to_schema(py),
        }
    }

    async fn execute(&self, params: HashMap<String, String>) -> String {
        match self {
            ToolType::ReadFile(t) => t.execute_inner(&params).await,
            ToolType::WriteFile(t) => t.execute_inner(&params).await,
            ToolType::EditFile(t) => t.execute_inner(&params).await,
            ToolType::ListDir(t) => t.execute_inner(&params).await,
            ToolType::Exec(t) => t.execute_inner(&params).await,
        }
    }
}

/// Registry for agent tools.
///
/// Allows dynamic registration and execution of tools.
#[pyclass]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, ToolType>>>,
}

#[pymethods]
impl ToolRegistry {
    #[new]
    fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a ReadFileTool.
    fn register_read_file(&self, tool: ReadFileTool) {
        let mut tools = futures::executor::block_on(self.tools.write());
        tools.insert(tool.tool_name().to_string(), ToolType::ReadFile(tool));
    }

    /// Register a WriteFileTool.
    fn register_write_file(&self, tool: WriteFileTool) {
        let mut tools = futures::executor::block_on(self.tools.write());
        tools.insert(tool.tool_name().to_string(), ToolType::WriteFile(tool));
    }

    /// Register an EditFileTool.
    fn register_edit_file(&self, tool: EditFileTool) {
        let mut tools = futures::executor::block_on(self.tools.write());
        tools.insert(tool.tool_name().to_string(), ToolType::EditFile(tool));
    }

    /// Register a ListDirTool.
    fn register_list_dir(&self, tool: ListDirTool) {
        let mut tools = futures::executor::block_on(self.tools.write());
        tools.insert(tool.tool_name().to_string(), ToolType::ListDir(tool));
    }

    /// Register an ExecTool.
    fn register_exec(&self, tool: ExecTool) {
        let mut tools = futures::executor::block_on(self.tools.write());
        tools.insert(tool.tool_name().to_string(), ToolType::Exec(tool));
    }

    /// Register any tool (generic method for Python compatibility).
    fn register(&self, tool: &Bound<'_, PyAny>) -> PyResult<()> {
        // Try to extract each tool type
        if let Ok(t) = tool.extract::<ReadFileTool>() {
            self.register_read_file(t);
            return Ok(());
        }
        if let Ok(t) = tool.extract::<WriteFileTool>() {
            self.register_write_file(t);
            return Ok(());
        }
        if let Ok(t) = tool.extract::<EditFileTool>() {
            self.register_edit_file(t);
            return Ok(());
        }
        if let Ok(t) = tool.extract::<ListDirTool>() {
            self.register_list_dir(t);
            return Ok(());
        }
        if let Ok(t) = tool.extract::<ExecTool>() {
            self.register_exec(t);
            return Ok(());
        }

        // For Python-based tools (web, message, spawn), we need to store them differently
        // For now, just ignore them - they'll be handled by Python fallback
        Ok(())
    }

    /// Unregister a tool by name.
    fn unregister(&self, name: &str) {
        let mut tools = futures::executor::block_on(self.tools.write());
        tools.remove(name);
    }

    /// Check if a tool is registered.
    fn has(&self, name: &str) -> bool {
        let tools = futures::executor::block_on(self.tools.read());
        tools.contains_key(name)
    }

    /// Get list of registered tool names.
    fn tool_names(&self) -> Vec<String> {
        let tools = futures::executor::block_on(self.tools.read());
        tools.keys().cloned().collect()
    }

    /// Get all tool definitions in OpenAI format.
    fn get_definitions(&self, py: Python<'_>) -> PyResult<PyObject> {
        let tools = futures::executor::block_on(self.tools.read());
        let list = PyList::empty(py);

        for tool in tools.values() {
            let schema = tool.to_schema(py)?;
            list.append(schema.to_dict(py)?)?;
        }

        Ok(list.into())
    }

    /// Execute a tool by name with given parameters.
    fn execute<'py>(
        &self,
        py: Python<'py>,
        name: String,
        params: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let tools = self.tools.clone();

        // Extract params to a HashMap<String, String>
        let mut param_map: HashMap<String, String> = HashMap::new();
        for (key, value) in params.iter() {
            let key_str: String = key.extract()?;
            // Try to extract as string, or convert to string
            let value_str: String = if let Ok(s) = value.extract::<String>() {
                s
            } else if let Ok(i) = value.extract::<i64>() {
                i.to_string()
            } else if let Ok(b) = value.extract::<bool>() {
                b.to_string()
            } else {
                value.str()?.to_string()
            };
            param_map.insert(key_str, value_str);
        }

        future_into_py(py, async move {
            let tools_guard = tools.read().await;

            if let Some(tool) = tools_guard.get(&name) {
                let tool = tool.clone();
                drop(tools_guard); // Release the lock before executing
                Ok(tool.execute(param_map).await)
            } else {
                Ok(format!("Error: Tool '{}' not found", name))
            }
        })
    }

    fn __len__(&self) -> usize {
        let tools = futures::executor::block_on(self.tools.read());
        tools.len()
    }

    fn __contains__(&self, name: &str) -> bool {
        self.has(name)
    }

    fn __repr__(&self) -> String {
        let tools = futures::executor::block_on(self.tools.read());
        format!("ToolRegistry(tools={})", tools.len())
    }
}
