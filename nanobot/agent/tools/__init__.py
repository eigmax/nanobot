"""Agent tools module - Rust implementation with Python fallback."""

# Always use Python ToolRegistry (handles Python-based tools like web, message, spawn)
from nanobot.agent.tools._base_py import Tool
from nanobot.agent.tools._registry_py import ToolRegistry

# Try to use Rust implementations for core tools (faster)
try:
    from nanobot_rust import (
        EditFileTool,
        ExecTool,
        ListDirTool,
        ReadFileTool,
        WebFetchTool,
        WebSearchTool,
        WriteFileTool,
    )
except ImportError:
    # Fallback to pure Python
    from nanobot.agent.tools._filesystem_py import (
        EditFileTool,
        ListDirTool,
        ReadFileTool,
        WriteFileTool,
    )
    from nanobot.agent.tools._shell_py import ExecTool
    from nanobot.agent.tools._web_py import WebFetchTool, WebSearchTool

# These stay in Python (depend on Python callbacks/state)
from nanobot.agent.tools.message import MessageTool
from nanobot.agent.tools.spawn import SpawnTool

__all__ = [
    "Tool",
    "ToolRegistry",
    "ReadFileTool",
    "WriteFileTool",
    "EditFileTool",
    "ListDirTool",
    "ExecTool",
    "WebSearchTool",
    "WebFetchTool",
    "MessageTool",
    "SpawnTool",
]
