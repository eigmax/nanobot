"""Tests for the tools module (Rust implementation)."""

import asyncio
import os
import tempfile
import pytest

from debot.agent.tools import (
    ToolRegistry,
    ReadFileTool,
    WriteFileTool,
    EditFileTool,
    ListDirTool,
    ExecTool,
)


class TestReadFileTool:
    """Tests for ReadFileTool."""

    @pytest.fixture
    def tool(self):
        return ReadFileTool()

    @pytest.mark.asyncio
    async def test_read_existing_file(self, tool):
        """Test reading an existing file."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False) as f:
            f.write("Hello, World!")
            f.flush()
            path = f.name

        try:
            result = await tool.execute(path)
            assert result == "Hello, World!"
        finally:
            os.unlink(path)

    @pytest.mark.asyncio
    async def test_read_nonexistent_file(self, tool):
        """Test reading a file that doesn't exist."""
        result = await tool.execute("/nonexistent/path/file.txt")
        assert "Error: File not found" in result

    def test_tool_properties(self, tool):
        """Test tool name and description."""
        assert tool.name == "read_file"
        assert "Read" in tool.description
        assert "path" in tool.parameters["required"]


class TestWriteFileTool:
    """Tests for WriteFileTool."""

    @pytest.fixture
    def tool(self):
        return WriteFileTool()

    @pytest.mark.asyncio
    async def test_write_new_file(self, tool):
        """Test writing a new file."""
        with tempfile.TemporaryDirectory() as tmpdir:
            path = os.path.join(tmpdir, "test.txt")
            result = await tool.execute(path, "Test content")

            assert "Successfully wrote" in result
            assert os.path.exists(path)
            with open(path) as f:
                assert f.read() == "Test content"

    @pytest.mark.asyncio
    async def test_write_creates_parent_dirs(self, tool):
        """Test that writing creates parent directories."""
        with tempfile.TemporaryDirectory() as tmpdir:
            path = os.path.join(tmpdir, "nested", "deep", "test.txt")
            result = await tool.execute(path, "Nested content")

            assert "Successfully wrote" in result
            assert os.path.exists(path)

    def test_tool_properties(self, tool):
        """Test tool name and description."""
        assert tool.name == "write_file"
        assert "path" in tool.parameters["required"]
        assert "content" in tool.parameters["required"]


class TestEditFileTool:
    """Tests for EditFileTool."""

    @pytest.fixture
    def tool(self):
        return EditFileTool()

    @pytest.mark.asyncio
    async def test_edit_file(self, tool):
        """Test editing a file."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False) as f:
            f.write("Hello, World!")
            f.flush()
            path = f.name

        try:
            result = await tool.execute(path, "World", "Rust")
            assert "Successfully edited" in result

            with open(path) as f:
                assert f.read() == "Hello, Rust!"
        finally:
            os.unlink(path)

    @pytest.mark.asyncio
    async def test_edit_text_not_found(self, tool):
        """Test editing when old_text is not found."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False) as f:
            f.write("Hello, World!")
            f.flush()
            path = f.name

        try:
            result = await tool.execute(path, "NotFound", "Replacement")
            assert "Error: old_text not found" in result
        finally:
            os.unlink(path)

    @pytest.mark.asyncio
    async def test_edit_multiple_occurrences(self, tool):
        """Test editing when old_text appears multiple times."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False) as f:
            f.write("foo bar foo")
            f.flush()
            path = f.name

        try:
            result = await tool.execute(path, "foo", "baz")
            assert "Warning" in result and "2 times" in result
        finally:
            os.unlink(path)

    def test_tool_properties(self, tool):
        """Test tool name and description."""
        assert tool.name == "edit_file"
        assert "path" in tool.parameters["required"]
        assert "old_text" in tool.parameters["required"]
        assert "new_text" in tool.parameters["required"]


class TestListDirTool:
    """Tests for ListDirTool."""

    @pytest.fixture
    def tool(self):
        return ListDirTool()

    @pytest.mark.asyncio
    async def test_list_directory(self, tool):
        """Test listing a directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create some files and directories
            os.makedirs(os.path.join(tmpdir, "subdir"))
            open(os.path.join(tmpdir, "file.txt"), 'w').close()

            result = await tool.execute(tmpdir)
            assert "file.txt" in result
            assert "subdir" in result

    @pytest.mark.asyncio
    async def test_list_empty_directory(self, tool):
        """Test listing an empty directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = await tool.execute(tmpdir)
            assert "empty" in result.lower()

    @pytest.mark.asyncio
    async def test_list_nonexistent_directory(self, tool):
        """Test listing a nonexistent directory."""
        result = await tool.execute("/nonexistent/path")
        assert "Error" in result

    def test_tool_properties(self, tool):
        """Test tool name and description."""
        assert tool.name == "list_dir"
        assert "path" in tool.parameters["required"]


class TestExecTool:
    """Tests for ExecTool."""

    @pytest.fixture
    def tool(self):
        return ExecTool(timeout=30)

    @pytest.mark.asyncio
    async def test_exec_simple_command(self, tool):
        """Test executing a simple command."""
        result = await tool.execute("echo 'Hello from shell'")
        assert "Hello from shell" in result

    @pytest.mark.asyncio
    async def test_exec_with_exit_code(self, tool):
        """Test command with non-zero exit code."""
        result = await tool.execute("exit 1")
        assert "Exit code: 1" in result

    @pytest.mark.asyncio
    async def test_exec_with_stderr(self, tool):
        """Test command that produces stderr."""
        result = await tool.execute("echo 'error' >&2")
        assert "STDERR" in result or "error" in result

    @pytest.mark.asyncio
    async def test_exec_with_working_dir(self, tool):
        """Test command with working directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = await tool.execute("pwd", working_dir=tmpdir)
            assert tmpdir in result

    @pytest.mark.asyncio
    async def test_exec_timeout(self):
        """Test command timeout."""
        tool = ExecTool(timeout=1)
        result = await tool.execute("sleep 10")
        assert "timed out" in result.lower()

    def test_tool_properties(self, tool):
        """Test tool name and description."""
        assert tool.name == "exec"
        assert "command" in tool.parameters["required"]


class TestToolRegistry:
    """Tests for ToolRegistry."""

    @pytest.fixture
    def registry(self):
        return ToolRegistry()

    def test_register_tools(self, registry):
        """Test registering tools."""
        registry.register(ReadFileTool())
        registry.register(WriteFileTool())
        registry.register(ExecTool())

        assert registry.has("read_file")
        assert registry.has("write_file")
        assert registry.has("exec")
        assert len(registry) == 3

    def test_tool_names(self, registry):
        """Test getting tool names."""
        registry.register(ReadFileTool())
        registry.register(EditFileTool())

        names = registry.tool_names
        assert "read_file" in names
        assert "edit_file" in names

    def test_unregister_tool(self, registry):
        """Test unregistering a tool."""
        registry.register(ReadFileTool())
        assert registry.has("read_file")

        registry.unregister("read_file")
        assert not registry.has("read_file")

    @pytest.mark.asyncio
    async def test_execute_tool(self, registry):
        """Test executing a tool via registry."""
        registry.register(ReadFileTool())

        with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False) as f:
            f.write("Registry test")
            f.flush()
            path = f.name

        try:
            result = await registry.execute("read_file", {"path": path})
            assert result == "Registry test"
        finally:
            os.unlink(path)

    @pytest.mark.asyncio
    async def test_execute_nonexistent_tool(self, registry):
        """Test executing a tool that doesn't exist."""
        result = await registry.execute("nonexistent", {})
        assert "Error" in result and "not found" in result

    def test_get_definitions(self, registry):
        """Test getting tool definitions."""
        registry.register(ReadFileTool())
        registry.register(ExecTool())

        defs = registry.get_definitions()
        assert len(defs) == 2

        # Check structure
        for d in defs:
            assert d["type"] == "function"
            assert "function" in d
            assert "name" in d["function"]
            assert "description" in d["function"]
            assert "parameters" in d["function"]

    def test_contains(self, registry):
        """Test __contains__ method."""
        registry.register(ReadFileTool())

        assert "read_file" in registry
        assert "nonexistent" not in registry


class TestPythonFallback:
    """Tests for Python fallback implementation."""

    def test_fallback_modules_exist(self):
        """Test that Python fallback modules can be imported."""
        from debot.agent.tools._base_py import Tool
        from debot.agent.tools._registry_py import ToolRegistry as PyRegistry
        from debot.agent.tools._filesystem_py import ReadFileTool as PyReadFile
        from debot.agent.tools._shell_py import ExecTool as PyExec

        assert Tool is not None
        assert PyRegistry is not None
        assert PyReadFile is not None
        assert PyExec is not None
