//! Memory system for persistent agent memory.

use pyo3::prelude::*;
use pyo3::types::PyList;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

/// Memory system for the agent.
///
/// Supports daily notes (memory/YYYY-MM-DD.md) and long-term memory (MEMORY.md).
#[pyclass]
pub struct MemoryStore {
    workspace: PathBuf,
    memory_dir: PathBuf,
    memory_file: PathBuf,
    index_file: PathBuf,
}

#[pymethods]
impl MemoryStore {
    #[new]
    pub fn new(workspace: PathBuf) -> PyResult<Self> {
        let memory_dir = workspace.join("memory");

        // Ensure directory exists
        fs::create_dir_all(&memory_dir).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to create memory directory: {}",
                e
            ))
        })?;

        let memory_file = memory_dir.join("MEMORY.md");
        let index_file = memory_dir.join(".index.json");

        Ok(MemoryStore {
            workspace,
            memory_dir,
            memory_file,
            index_file,
        })
    }

    /// Get path to today's memory file.
    fn get_today_file(&self) -> String {
        let today = today_date();
        self.memory_dir
            .join(format!("{}.md", today))
            .to_string_lossy()
            .to_string()
    }

    /// Read today's memory notes.
    fn read_today(&self) -> String {
        let today_file = self.memory_dir.join(format!("{}.md", today_date()));
        if today_file.exists() {
            fs::read_to_string(&today_file).unwrap_or_default()
        } else {
            String::new()
        }
    }

    /// Append content to today's memory notes.
    fn append_today(&self, content: String) -> PyResult<()> {
        let today = today_date();
        let today_file = self.memory_dir.join(format!("{}.md", today));

        let final_content = if today_file.exists() {
            let existing = fs::read_to_string(&today_file).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                    "Failed to read today's file: {}",
                    e
                ))
            })?;
            format!("{}\n{}", existing, content)
        } else {
            // Add header for new day
            format!("# {}\n\n{}", today, content)
        };

        fs::write(&today_file, final_content).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to write today's file: {}",
                e
            ))
        })?;

        Ok(())
    }

    /// Read long-term memory (MEMORY.md).
    fn read_long_term(&self) -> String {
        if self.memory_file.exists() {
            fs::read_to_string(&self.memory_file).unwrap_or_default()
        } else {
            String::new()
        }
    }

    /// Write to long-term memory (MEMORY.md).
    fn write_long_term(&self, content: String) -> PyResult<()> {
        fs::write(&self.memory_file, content).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to write long-term memory: {}",
                e
            ))
        })?;
        Ok(())
    }

    /// Get memories from the last N days.
    #[pyo3(signature = (days=7))]
    fn get_recent_memories(&self, days: i64) -> String {
        use chrono::{Duration, Local};

        let today = Local::now().date_naive();
        let mut memories = Vec::new();

        for i in 0..days {
            let date = today - Duration::days(i);
            let date_str = date.format("%Y-%m-%d").to_string();
            let file_path = self.memory_dir.join(format!("{}.md", date_str));

            if file_path.exists() {
                if let Ok(content) = fs::read_to_string(&file_path) {
                    memories.push(content);
                }
            }
        }

        memories.join("\n\n---\n\n")
    }

    /// List all memory files sorted by date (newest first).
    fn list_memory_files(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let result = PyList::empty(py);

        if !self.memory_dir.exists() {
            return Ok(result.into());
        }

        let mut files: Vec<PathBuf> = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.memory_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Match pattern YYYY-MM-DD.md
                    if name.len() == 13
                        && name.ends_with(".md")
                        && name.chars().nth(4) == Some('-')
                        && name.chars().nth(7) == Some('-')
                    {
                        files.push(path);
                    }
                }
            }
        }

        // Sort by filename (date) descending
        files.sort_by(|a, b| b.cmp(a));

        for file in files {
            result.append(file.to_string_lossy().to_string())?;
        }

        Ok(result.into())
    }

    /// Build a simple, local vector index for all markdown memory files.
    /// This uses a deterministic local embedding (SHA256-based) so no external API is required.
    pub fn build_index(&self) -> PyResult<usize> {
        let mut entries: Vec<IndexEntry> = Vec::new();

        if !self.memory_dir.exists() {
            return Ok(0);
        }

        if let Ok(entries_iter) = fs::read_dir(&self.memory_dir) {
            for entry in entries_iter.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.ends_with(".md") {
                        if let Ok(text) = fs::read_to_string(&path) {
                            // Chunk by roughly 800 characters with 100 char overlap
                            let chunk_size = 800;
                            let overlap = 100;
                            let mut start = 0usize;
                            let len = text.len();
                            while start < len {
                                let end = if start + chunk_size > len {
                                    len
                                } else {
                                    start + chunk_size
                                };
                                let chunk = &text[start..end];
                                let vec = embed_text(chunk);
                                let id = Uuid::new_v4().to_string();
                                let entry = IndexEntry {
                                    id,
                                    path: path
                                        .strip_prefix(&self.workspace)
                                        .unwrap_or(&path)
                                        .to_string_lossy()
                                        .to_string(),
                                    start_line: 0,
                                    end_line: 0,
                                    text: chunk.to_string(),
                                    vector: vec,
                                };
                                entries.push(entry);
                                if end == len {
                                    break;
                                }
                                start = end.saturating_sub(overlap);
                            }
                        }
                    }
                }
            }
        }

        // Serialize index to file
        let json = serde_json::to_string_pretty(&entries).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to serialize index: {}",
                e
            ))
        })?;

        let mut f = fs::File::create(&self.index_file).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to create index file: {}",
                e
            ))
        })?;
        f.write_all(json.as_bytes()).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to write index file: {}",
                e
            ))
        })?;

        Ok(entries.len())
    }

    /// Search the local index for semantically similar chunks to `query`.
    /// Returns a list of dict-like tuples: (path, snippet, score)
    #[pyo3(signature = (query, max_results=5, min_score=0.0))]
    pub fn search(
        &self,
        py: Python<'_>,
        query: String,
        max_results: usize,
        min_score: f32,
    ) -> PyResult<Py<PyList>> {
        #[allow(unused_mut)]
        let mut result = PyList::empty(py);

        if !self.index_file.exists() {
            return Ok(result.into());
        }

        let json = fs::read_to_string(&self.index_file).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to read index file: {}",
                e
            ))
        })?;

        let entries: Vec<IndexEntry> = serde_json::from_str(&json).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to parse index: {}", e))
        })?;

        let qvec = embed_text(&query);

        let mut scored: Vec<(f32, &IndexEntry)> = entries
            .iter()
            .map(|e| {
                let score = cosine_similarity(&qvec, &e.vector);
                (score, e)
            })
            .collect();

        scored.retain(|(s, _)| *s >= min_score);
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        for (score, entry) in scored.into_iter().take(max_results) {
            let dict = pyo3::types::PyDict::new(py);
            dict.set_item("path", entry.path.clone())?;
            dict.set_item("snippet", entry.text.clone())?;
            dict.set_item("score", score)?;
            result.append(dict)?;
        }

        Ok(result.into())
    }

    /// Get memory context for the agent.
    pub fn get_memory_context(&self) -> String {
        let mut parts = Vec::new();

        // Long-term memory
        let long_term = self.read_long_term();
        if !long_term.is_empty() {
            parts.push(format!("## Long-term Memory\n{}", long_term));
        }

        // Today's notes
        let today = self.read_today();
        if !today.is_empty() {
            parts.push(format!("## Today's Notes\n{}", today));
        }

        if parts.is_empty() {
            String::new()
        } else {
            parts.join("\n\n")
        }
    }

    /// Get the workspace path.
    #[getter]
    fn workspace(&self) -> String {
        self.workspace.to_string_lossy().to_string()
    }

    /// Get the memory directory path.
    #[getter]
    fn memory_dir(&self) -> String {
        self.memory_dir.to_string_lossy().to_string()
    }

    /// Get the memory file path.
    #[getter]
    fn memory_file(&self) -> String {
        self.memory_file.to_string_lossy().to_string()
    }
}

/// Get today's date in YYYY-MM-DD format.
fn today_date() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[derive(Serialize, Deserialize)]
struct IndexEntry {
    id: String,
    path: String,
    start_line: usize,
    end_line: usize,
    text: String,
    vector: Vec<f32>,
}

/// Create a deterministic local embedding for `text` using SHA256.
/// This is a placeholder for a real embedding API and yields a fixed-length vector.
fn embed_local(text: &str) -> Vec<f32> {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hasher.finalize();
    // Build a 64-dim vector by repeating/expanding the digest bytes
    let dims = 64usize;
    let mut vec: Vec<f32> = Vec::with_capacity(dims);
    for i in 0..dims {
        let b = digest[i % digest.len()];
        // map byte (0..255) to -1.0 .. 1.0
        let v = (b as f32 / 127.5) - 1.0;
        vec.push(v);
    }
    // normalize
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    for v in &mut vec {
        *v /= norm;
    }
    vec
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    dot
}

/// Try remote embedding via OpenAI-compatible endpoint (OPENAI_API_BASE/OPENAI_API_KEY or OPENROUTER_API_KEY).
/// Returns None on any failure; caller should fall back to local embedding.
fn embed_remote(text: &str) -> Option<Vec<f32>> {
    let api_key = env::var("OPENAI_API_KEY")
        .ok()
        .or_else(|| env::var("OPENROUTER_API_KEY").ok())?;

    let api_base =
        env::var("OPENAI_API_BASE").unwrap_or_else(|_| "https://api.openai.com".to_string());
    let url = format!("{}/v1/embeddings", api_base.trim_end_matches('/'));

    let client = Client::new();
    let body = serde_json::json!({"model": "text-embedding-3-small", "input": text});

    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: Value = resp.json().ok()?;
    let arr = v.get("data")?.get(0)?.get("embedding")?.as_array()?;
    let vec: Vec<f32> = arr
        .iter()
        .filter_map(|n| n.as_f64().map(|f| f as f32))
        .collect();
    if vec.is_empty() {
        None
    } else {
        Some(vec)
    }
}

/// Embed text using remote provider when available, otherwise fall back to deterministic local embedding.
fn embed_text(text: &str) -> Vec<f32> {
    if let Some(v) = embed_remote(text) {
        v
    } else {
        embed_local(text)
    }
}
