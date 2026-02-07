//! Context builder for assembling agent prompts.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::fs;
use std::path::PathBuf;

use crate::memory::MemoryStore;
use crate::skills::SkillsLoader;

/// Bootstrap files to load from workspace.
const BOOTSTRAP_FILES: &[&str] = &["AGENTS.md", "SOUL.md", "USER.md", "TOOLS.md", "IDENTITY.md"];

/// Builds the context (system prompt + messages) for the agent.
///
/// Assembles bootstrap files, memory, skills, and conversation history
/// into a coherent prompt for the LLM.
#[pyclass]
pub struct ContextBuilder {
    workspace: PathBuf,
    memory: MemoryStore,
    skills: SkillsLoader,
}

#[pymethods]
impl ContextBuilder {
    #[new]
    fn new(workspace: PathBuf) -> PyResult<Self> {
        let memory = MemoryStore::new(workspace.clone())?;
        let skills = SkillsLoader::new(workspace.clone(), None);

        Ok(ContextBuilder {
            workspace,
            memory,
            skills,
        })
    }

    /// Build the system prompt from bootstrap files, memory, and skills.
    #[pyo3(signature = (skill_names=None))]
    #[allow(unused_variables)]
    fn build_system_prompt(
        &self,
        py: Python<'_>,
        skill_names: Option<Vec<String>>,
    ) -> PyResult<String> {
        let mut parts = Vec::new();

        // Core identity
        parts.push(self.get_identity());

        // Bootstrap files
        let bootstrap = self.load_bootstrap_files();
        if !bootstrap.is_empty() {
            parts.push(bootstrap);
        }

        // Memory context
        let memory = self.memory.get_memory_context();
        if !memory.is_empty() {
            parts.push(format!("# Memory\n\n{}", memory));
        }

        // Skills - progressive loading
        // 1. Always-loaded skills: include full content
        let always_skills = self.skills.get_always_skills(py)?;
        if !always_skills.is_empty() {
            let always_content = self.skills.load_skills_for_context(always_skills);
            if !always_content.is_empty() {
                parts.push(format!("# Active Skills\n\n{}", always_content));
            }
        }

        // 2. Available skills: only show summary (agent uses read_file to load)
        let skills_summary = self.skills.build_skills_summary(py)?;
        if !skills_summary.is_empty() {
            parts.push(format!(
                "# Skills\n\n\
                The following skills extend your capabilities. To use a skill, read its SKILL.md file using the read_file tool.\n\
                Skills with available=\"false\" need dependencies installed first - you can try installing them with apt/brew.\n\n\
                {}",
                skills_summary
            ));
        }

        Ok(parts.join("\n\n---\n\n"))
    }

    /// Build the complete message list for an LLM call.
    #[pyo3(signature = (history, current_message, skill_names=None, media=None))]
    fn build_messages(
        &self,
        py: Python<'_>,
        history: &Bound<'_, PyList>,
        current_message: &str,
        skill_names: Option<Vec<String>>,
        media: Option<Vec<String>>,
    ) -> PyResult<Py<PyList>> {
        let messages = PyList::empty(py);

        // System prompt
        let system_prompt = self.build_system_prompt(py, skill_names)?;
        let system_msg = PyDict::new(py);
        system_msg.set_item("role", "system")?;
        system_msg.set_item("content", system_prompt)?;
        messages.append(system_msg)?;

        // History
        for item in history.iter() {
            messages.append(item)?;
        }

        // Current message (with optional image attachments)
        let user_content = self.build_user_content(py, current_message, media)?;
        let user_msg = PyDict::new(py);
        user_msg.set_item("role", "user")?;
        user_msg.set_item("content", user_content)?;
        messages.append(user_msg)?;

        Ok(messages.into())
    }

    /// Add a tool result to the message list.
    fn add_tool_result(
        &self,
        py: Python<'_>,
        messages: &Bound<'_, PyList>,
        tool_call_id: &str,
        tool_name: &str,
        result: &str,
    ) -> PyResult<Py<PyList>> {
        let msg = PyDict::new(py);
        msg.set_item("role", "tool")?;
        msg.set_item("tool_call_id", tool_call_id)?;
        msg.set_item("name", tool_name)?;
        msg.set_item("content", result)?;
        messages.append(msg)?;

        Ok(messages.clone().unbind())
    }

    /// Add an assistant message to the message list.
    #[pyo3(signature = (messages, content, tool_calls=None))]
    fn add_assistant_message(
        &self,
        py: Python<'_>,
        messages: &Bound<'_, PyList>,
        content: Option<&str>,
        tool_calls: Option<&Bound<'_, PyList>>,
    ) -> PyResult<Py<PyList>> {
        let msg = PyDict::new(py);
        msg.set_item("role", "assistant")?;
        msg.set_item("content", content.unwrap_or(""))?;

        if let Some(tc) = tool_calls {
            msg.set_item("tool_calls", tc)?;
        }

        messages.append(msg)?;
        Ok(messages.clone().unbind())
    }

    /// Get the workspace path.
    #[getter]
    fn workspace(&self) -> String {
        self.workspace.to_string_lossy().to_string()
    }
}

impl ContextBuilder {
    fn get_identity(&self) -> String {
        let now = chrono::Local::now()
            .format("%Y-%m-%d %H:%M (%A)")
            .to_string();
        let workspace_path = self
            .workspace
            .canonicalize()
            .unwrap_or_else(|_| self.workspace.clone())
            .to_string_lossy()
            .to_string();

        format!(
            r#"# debot 🐈

You are debot, a helpful AI assistant. You have access to tools that allow you to:
- Read, write, and edit files
- Execute shell commands
- Search the web and fetch web pages
- Send messages to users on chat channels
- Spawn subagents for complex background tasks

## Current Time
{}

## Workspace
Your workspace is at: {}
- Memory files: {}/memory/MEMORY.md
- Daily notes: {}/memory/YYYY-MM-DD.md
- Custom skills: {}/skills/{{skill-name}}/SKILL.md

IMPORTANT: When responding to direct questions or conversations, reply directly with your text response.
Only use the 'message' tool when you need to send a message to a specific chat channel (like WhatsApp).
For normal conversation, just respond with text - do not call the message tool.

Always be helpful, accurate, and concise. When using tools, explain what you're doing.
When remembering something, write to {}/memory/MEMORY.md"#,
            now, workspace_path, workspace_path, workspace_path, workspace_path, workspace_path
        )
    }

    fn load_bootstrap_files(&self) -> String {
        let mut parts = Vec::new();

        for filename in BOOTSTRAP_FILES {
            let file_path = self.workspace.join(filename);
            if file_path.exists() {
                if let Ok(content) = fs::read_to_string(&file_path) {
                    parts.push(format!("## {}\n\n{}", filename, content));
                }
            }
        }

        if parts.is_empty() {
            String::new()
        } else {
            parts.join("\n\n")
        }
    }

    fn build_user_content(
        &self,
        py: Python<'_>,
        text: &str,
        media: Option<Vec<String>>,
    ) -> PyResult<PyObject> {
        let media = match media {
            Some(m) if !m.is_empty() => m,
            _ => return Ok(text.into_pyobject(py)?.into_any().unbind()),
        };

        let mut images = Vec::new();

        for path in &media {
            let p = PathBuf::from(path);
            if !p.is_file() {
                continue;
            }

            let mime = guess_mime_type(path);
            if !mime.starts_with("image/") {
                continue;
            }

            if let Ok(bytes) = fs::read(&p) {
                let b64 = BASE64.encode(&bytes);
                let image_dict = PyDict::new(py);
                image_dict.set_item("type", "image_url")?;

                let url_dict = PyDict::new(py);
                url_dict.set_item("url", format!("data:{};base64,{}", mime, b64))?;
                image_dict.set_item("image_url", url_dict)?;

                images.push(image_dict);
            }
        }

        if images.is_empty() {
            return Ok(text.into_pyobject(py)?.into_any().unbind());
        }

        // Build content array: images + text
        let content = PyList::empty(py);
        for img in images {
            content.append(img)?;
        }

        let text_dict = PyDict::new(py);
        text_dict.set_item("type", "text")?;
        text_dict.set_item("text", text)?;
        content.append(text_dict)?;

        Ok(content.into())
    }
}

/// Guess MIME type from file extension.
fn guess_mime_type(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "png" => "image/png".to_string(),
        "gif" => "image/gif".to_string(),
        "webp" => "image/webp".to_string(),
        "svg" => "image/svg+xml".to_string(),
        "bmp" => "image/bmp".to_string(),
        "ico" => "image/x-icon".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}
