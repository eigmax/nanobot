"""Tests for the memory module (Rust implementation)."""

import tempfile
from pathlib import Path
from datetime import datetime

import pytest

from debot.agent.memory import MemoryStore


class TestMemoryStore:
    """Tests for MemoryStore class."""

    @pytest.fixture
    def temp_workspace(self):
        """Create a temporary workspace directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            yield Path(tmpdir)

    @pytest.fixture
    def store(self, temp_workspace):
        """Create a MemoryStore with temp workspace."""
        return MemoryStore(temp_workspace)

    def test_create_memory_store(self, temp_workspace):
        """Test creating a new memory store."""
        store = MemoryStore(temp_workspace)
        # Memory directory should be created
        memory_dir = temp_workspace / "memory"
        assert memory_dir.exists()

    def test_get_today_file(self, store):
        """Test getting today's file path."""
        today_file = store.get_today_file()
        today = datetime.now().strftime("%Y-%m-%d")
        assert today in today_file
        assert today_file.endswith(".md")

    def test_read_today_empty(self, store):
        """Test reading today's notes when empty."""
        content = store.read_today()
        assert content == ""

    def test_append_today(self, store):
        """Test appending to today's notes."""
        store.append_today("First note")

        content = store.read_today()
        assert "First note" in content
        # Should have date header
        today = datetime.now().strftime("%Y-%m-%d")
        assert today in content

    def test_append_today_multiple(self, store):
        """Test appending multiple notes to today."""
        store.append_today("First note")
        store.append_today("Second note")

        content = store.read_today()
        assert "First note" in content
        assert "Second note" in content

    def test_read_long_term_empty(self, store):
        """Test reading long-term memory when empty."""
        content = store.read_long_term()
        assert content == ""

    def test_write_and_read_long_term(self, store):
        """Test writing and reading long-term memory."""
        store.write_long_term("# Long-term Memory\n\nImportant fact.")

        content = store.read_long_term()
        assert "Long-term Memory" in content
        assert "Important fact" in content

    def test_write_long_term_overwrites(self, store):
        """Test that write_long_term overwrites existing content."""
        store.write_long_term("First content")
        store.write_long_term("Second content")

        content = store.read_long_term()
        assert "First content" not in content
        assert "Second content" in content

    def test_get_recent_memories_empty(self, store):
        """Test getting recent memories when none exist."""
        content = store.get_recent_memories(days=7)
        assert content == ""

    def test_get_recent_memories(self, store, temp_workspace):
        """Test getting recent memories."""
        # Create today's memory
        store.append_today("Today's memory")

        # Get recent memories (should include today)
        content = store.get_recent_memories(days=7)
        assert "Today's memory" in content

    def test_list_memory_files_empty(self, store):
        """Test listing memory files when none exist."""
        files = store.list_memory_files()
        assert len(files) == 0

    def test_list_memory_files(self, store, temp_workspace):
        """Test listing memory files."""
        # Create some memory files
        memory_dir = temp_workspace / "memory"
        (memory_dir / "2024-01-01.md").write_text("Day 1")
        (memory_dir / "2024-01-02.md").write_text("Day 2")
        (memory_dir / "2024-01-03.md").write_text("Day 3")

        files = store.list_memory_files()
        assert len(files) == 3
        # Should be sorted newest first
        assert "2024-01-03" in files[0]
        assert "2024-01-01" in files[2]

    def test_list_memory_files_ignores_other_files(self, store, temp_workspace):
        """Test that list_memory_files ignores non-date files."""
        memory_dir = temp_workspace / "memory"
        (memory_dir / "2024-01-01.md").write_text("Day 1")
        (memory_dir / "MEMORY.md").write_text("Long term")
        (memory_dir / "notes.txt").write_text("Other")

        files = store.list_memory_files()
        assert len(files) == 1
        assert "2024-01-01" in files[0]

    def test_get_memory_context_empty(self, store):
        """Test getting memory context when empty."""
        context = store.get_memory_context()
        assert context == ""

    def test_get_memory_context_with_long_term(self, store):
        """Test memory context with long-term memory."""
        store.write_long_term("Important knowledge")

        context = store.get_memory_context()
        assert "Long-term Memory" in context
        assert "Important knowledge" in context

    def test_get_memory_context_with_today(self, store):
        """Test memory context with today's notes."""
        store.append_today("Today's task")

        context = store.get_memory_context()
        assert "Today's Notes" in context
        assert "Today's task" in context

    def test_get_memory_context_with_both(self, store):
        """Test memory context with both long-term and today's notes."""
        store.write_long_term("Long-term fact")
        store.append_today("Today's note")

        context = store.get_memory_context()
        assert "Long-term Memory" in context
        assert "Long-term fact" in context
        assert "Today's Notes" in context
        assert "Today's note" in context

    def test_workspace_property(self, store, temp_workspace):
        """Test workspace property."""
        assert str(temp_workspace) in store.workspace

    def test_memory_dir_property(self, store, temp_workspace):
        """Test memory_dir property."""
        assert str(temp_workspace) in store.memory_dir
        assert "memory" in store.memory_dir

    def test_memory_file_property(self, store, temp_workspace):
        """Test memory_file property."""
        assert str(temp_workspace) in store.memory_file
        assert "MEMORY.md" in store.memory_file


class TestPythonFallback:
    """Tests for Python fallback implementation."""

    def test_fallback_import(self):
        """Test that Python fallback module can be imported."""
        from debot.agent._memory_py import MemoryStore as PyMemoryStore

        assert PyMemoryStore is not None
