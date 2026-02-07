use pyo3::prelude::*;

mod bus;
mod context;
mod cron;
mod heartbeat;
mod memory;
mod messages;
mod session;
mod skills;
mod tools;

use bus::MessageBus;
use context::ContextBuilder;
use cron::{CronJob, CronJobState, CronPayload, CronSchedule, CronService};
use heartbeat::HeartbeatService;
use memory::MemoryStore;
use messages::{InboundMessage, OutboundMessage};
use session::{Session, SessionManager};
use skills::SkillsLoader;
use tools::{
    EditFileTool, ExecTool, ListDirTool, ReadFileTool, ToolRegistry, WebFetchTool, WebSearchTool,
    WriteFileTool,
};

/// Rust implementation of debot core modules.
#[pymodule]
fn debot_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
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
    m.add_class::<WebSearchTool>()?;
    m.add_class::<WebFetchTool>()?;

    // Session classes
    m.add_class::<Session>()?;
    m.add_class::<SessionManager>()?;

    // Memory classes
    m.add_class::<MemoryStore>()?;

    // Skills and Context classes
    m.add_class::<SkillsLoader>()?;
    m.add_class::<ContextBuilder>()?;

    // Heartbeat service
    m.add_class::<HeartbeatService>()?;

    // Cron service
    m.add_class::<CronService>()?;
    m.add_class::<CronJob>()?;
    m.add_class::<CronSchedule>()?;
    m.add_class::<CronPayload>()?;
    m.add_class::<CronJobState>()?;

    Ok(())
}
