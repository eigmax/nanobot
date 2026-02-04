use pyo3::prelude::*;

mod bus;
mod context;
mod memory;
mod messages;
mod session;
mod skills;
mod tools;

use bus::MessageBus;
use context::ContextBuilder;
use memory::MemoryStore;
use messages::{InboundMessage, OutboundMessage};
use session::{Session, SessionManager};
use skills::SkillsLoader;
use tools::{EditFileTool, ExecTool, ListDirTool, ReadFileTool, ToolRegistry, WriteFileTool};

/// Rust implementation of nanobot core modules.
#[pymodule]
fn nanobot_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Message bus classes
    m.add_class::<InboundMessage>()?;
    m.add_class::<OutboundMessage>()?;
    m.add_class::<MessageBus>()?;

    // Tool classes
    m.add_class::<ToolRegistry>()?;
    m.add_class::<ReadFileTool>()?;
    m.add_class::<WriteFileTool>()?;
    m.add_class::<EditFileTool>()?;
    m.add_class::<ListDirTool>()?;
    m.add_class::<ExecTool>()?;

    // Session classes
    m.add_class::<Session>()?;
    m.add_class::<SessionManager>()?;

    // Memory classes
    m.add_class::<MemoryStore>()?;

    // Skills and Context classes
    m.add_class::<SkillsLoader>()?;
    m.add_class::<ContextBuilder>()?;

    Ok(())
}
