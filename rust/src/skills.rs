//! Skills loader for agent capabilities.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Loader for agent skills.
///
/// Skills are markdown files (SKILL.md) that teach the agent how to use
/// specific tools or perform certain tasks.
#[pyclass]
#[allow(dead_code)]
pub struct SkillsLoader {
    workspace: PathBuf,
    workspace_skills: PathBuf,
    builtin_skills: PathBuf,
}

#[pymethods]
impl SkillsLoader {
    #[new]
    #[pyo3(signature = (workspace, builtin_skills_dir=None))]
    pub fn new(workspace: PathBuf, builtin_skills_dir: Option<PathBuf>) -> Self {
        let workspace_skills = workspace.join("skills");

        // Default builtin skills directory - relative to debot package
        let builtin_skills = builtin_skills_dir.unwrap_or_else(|| {
            // Try to find the builtin skills directory
            // This would be debot/skills relative to the package
            PathBuf::from("")
        });

        SkillsLoader {
            workspace,
            workspace_skills,
            builtin_skills,
        }
    }

    /// List all available skills.
    #[pyo3(signature = (filter_unavailable=true))]
    fn list_skills(&self, py: Python<'_>, filter_unavailable: bool) -> PyResult<Py<PyList>> {
        let result = PyList::empty(py);
        let mut seen_names: Vec<String> = Vec::new();

        // Workspace skills (highest priority)
        if self.workspace_skills.exists() {
            if let Ok(entries) = fs::read_dir(&self.workspace_skills) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let skill_file = path.join("SKILL.md");
                        if skill_file.exists() {
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                let dict = PyDict::new(py);
                                dict.set_item("name", name)?;
                                dict.set_item("path", skill_file.to_string_lossy().to_string())?;
                                dict.set_item("source", "workspace")?;

                                if !filter_unavailable || self.check_requirements_for_skill(name) {
                                    result.append(dict)?;
                                    seen_names.push(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Built-in skills
        if !self.builtin_skills.as_os_str().is_empty() && self.builtin_skills.exists() {
            if let Ok(entries) = fs::read_dir(&self.builtin_skills) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let skill_file = path.join("SKILL.md");
                        if skill_file.exists() {
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                if !seen_names.contains(&name.to_string()) {
                                    let dict = PyDict::new(py);
                                    dict.set_item("name", name)?;
                                    dict.set_item(
                                        "path",
                                        skill_file.to_string_lossy().to_string(),
                                    )?;
                                    dict.set_item("source", "builtin")?;

                                    if !filter_unavailable
                                        || self.check_requirements_for_skill(name)
                                    {
                                        result.append(dict)?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(result.into())
    }

    /// Load a skill by name.
    fn load_skill(&self, name: &str) -> Option<String> {
        // Check workspace first
        let workspace_skill = self.workspace_skills.join(name).join("SKILL.md");
        if workspace_skill.exists() {
            return fs::read_to_string(&workspace_skill).ok();
        }

        // Check built-in
        if !self.builtin_skills.as_os_str().is_empty() {
            let builtin_skill = self.builtin_skills.join(name).join("SKILL.md");
            if builtin_skill.exists() {
                return fs::read_to_string(&builtin_skill).ok();
            }
        }

        None
    }

    /// Load specific skills for inclusion in agent context.
    pub fn load_skills_for_context(&self, skill_names: Vec<String>) -> String {
        let mut parts = Vec::new();

        for name in skill_names {
            if let Some(content) = self.load_skill(&name) {
                let stripped = strip_frontmatter(&content);
                parts.push(format!("### Skill: {}\n\n{}", name, stripped));
            }
        }

        if parts.is_empty() {
            String::new()
        } else {
            parts.join("\n\n---\n\n")
        }
    }

    /// Build a summary of all skills (name, description, path, availability).
    pub fn build_skills_summary(&self, py: Python<'_>) -> PyResult<String> {
        let all_skills = self.list_skills(py, false)?;
        let skills_list = all_skills.bind(py);

        if skills_list.len() == 0 {
            return Ok(String::new());
        }

        fn escape_xml(s: &str) -> String {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        }

        let mut lines = vec!["<skills>".to_string()];

        for item in skills_list.iter() {
            let dict = item.downcast::<PyDict>()?;
            let name: String = dict.get_item("name")?.unwrap().extract()?;
            let path: String = dict.get_item("path")?.unwrap().extract()?;

            let desc = self.get_skill_description(&name);
            let skill_meta = self.get_skill_meta(&name);
            let available = self.check_requirements(&skill_meta);

            lines.push(format!(
                "  <skill available=\"{}\">",
                available.to_string().to_lowercase()
            ));
            lines.push(format!("    <name>{}</name>", escape_xml(&name)));
            lines.push(format!(
                "    <description>{}</description>",
                escape_xml(&desc)
            ));
            lines.push(format!("    <location>{}</location>", path));

            if !available {
                let missing = self.get_missing_requirements(&skill_meta);
                if !missing.is_empty() {
                    lines.push(format!("    <requires>{}</requires>", escape_xml(&missing)));
                }
            }

            lines.push("  </skill>".to_string());
        }

        lines.push("</skills>".to_string());
        Ok(lines.join("\n"))
    }

    /// Get skills marked as always=true that meet requirements.
    pub fn get_always_skills(&self, py: Python<'_>) -> PyResult<Vec<String>> {
        let mut result = Vec::new();
        let skills = self.list_skills(py, true)?;
        let skills_list = skills.bind(py);

        for item in skills_list.iter() {
            let dict = item.downcast::<PyDict>()?;
            let name: String = dict.get_item("name")?.unwrap().extract()?;

            if let Some(meta) = self.get_skill_metadata(&name) {
                let skill_meta =
                    parse_debot_metadata(meta.get("metadata").cloned().as_deref().unwrap_or(""));
                if skill_meta
                    .get("always")
                    .map(|v| v == "true")
                    .unwrap_or(false)
                    || meta.get("always").map(|v| v == "true").unwrap_or(false)
                {
                    result.push(name);
                }
            }
        }

        Ok(result)
    }

    /// Get metadata from a skill's frontmatter.
    fn get_skill_metadata(&self, name: &str) -> Option<HashMap<String, String>> {
        let content = self.load_skill(name)?;

        if content.starts_with("---") {
            // Use (?s) flag to make . match newlines
            let re = Regex::new(r"(?s)^---\n(.*?)\n---").ok()?;
            if let Some(caps) = re.captures(&content) {
                let frontmatter = caps.get(1)?.as_str();
                let mut metadata = HashMap::new();

                for line in frontmatter.lines() {
                    if let Some(idx) = line.find(':') {
                        let key = line[..idx].trim().to_string();
                        let value = line[idx + 1..]
                            .trim()
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_string();
                        metadata.insert(key, value);
                    }
                }

                return Some(metadata);
            }
        }

        None
    }
}

impl SkillsLoader {
    fn get_skill_description(&self, name: &str) -> String {
        if let Some(meta) = self.get_skill_metadata(name) {
            if let Some(desc) = meta.get("description") {
                return desc.clone();
            }
        }
        name.to_string()
    }

    fn get_skill_meta(&self, name: &str) -> HashMap<String, String> {
        if let Some(meta) = self.get_skill_metadata(name) {
            if let Some(metadata_str) = meta.get("metadata") {
                return parse_debot_metadata(metadata_str);
            }
        }
        HashMap::new()
    }

    fn check_requirements_for_skill(&self, name: &str) -> bool {
        let skill_meta = self.get_skill_meta(name);
        self.check_requirements(&skill_meta)
    }

    fn check_requirements(&self, skill_meta: &HashMap<String, String>) -> bool {
        // Check bins
        if let Some(bins) = skill_meta.get("requires.bins") {
            for bin in bins.split(',').map(|s| s.trim()) {
                if !bin.is_empty() && !command_exists(bin) {
                    return false;
                }
            }
        }

        // Check env vars
        if let Some(envs) = skill_meta.get("requires.env") {
            for env_var in envs.split(',').map(|s| s.trim()) {
                if !env_var.is_empty() && env::var(env_var).is_err() {
                    return false;
                }
            }
        }

        true
    }

    fn get_missing_requirements(&self, skill_meta: &HashMap<String, String>) -> String {
        let mut missing = Vec::new();

        if let Some(bins) = skill_meta.get("requires.bins") {
            for bin in bins.split(',').map(|s| s.trim()) {
                if !bin.is_empty() && !command_exists(bin) {
                    missing.push(format!("CLI: {}", bin));
                }
            }
        }

        if let Some(envs) = skill_meta.get("requires.env") {
            for env_var in envs.split(',').map(|s| s.trim()) {
                if !env_var.is_empty() && env::var(env_var).is_err() {
                    missing.push(format!("ENV: {}", env_var));
                }
            }
        }

        missing.join(", ")
    }
}

/// Strip YAML frontmatter from markdown content.
fn strip_frontmatter(content: &str) -> String {
    if content.starts_with("---") {
        // Use (?s) flag to make . match newlines
        if let Ok(re) = Regex::new(r"(?s)^---\n.*?\n---\n") {
            if let Some(m) = re.find(content) {
                return content[m.end()..].trim().to_string();
            }
        }
    }
    content.to_string()
}

/// Parse debot metadata JSON from frontmatter.
fn parse_debot_metadata(raw: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    // Try to parse as JSON
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(obj) = value.get("debot").and_then(|v| v.as_object()) {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    result.insert(k.clone(), s.to_string());
                } else if let Some(b) = v.as_bool() {
                    result.insert(k.clone(), b.to_string());
                }
            }
        }
    }

    result
}

/// Check if a command exists in PATH.
fn command_exists(cmd: &str) -> bool {
    if let Ok(path) = env::var("PATH") {
        for dir in path.split(':') {
            let full_path = PathBuf::from(dir).join(cmd);
            if full_path.exists() && full_path.is_file() {
                return true;
            }
        }
    }
    false
}
