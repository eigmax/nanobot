"""Tests for the session module (Rust implementation)."""

import os
import tempfile
import uuid
from pathlib import Path

import pytest

from debot.session import Session, SessionManager


def unique_key(prefix: str = "test") -> str:
    """Generate a unique session key for testing."""
    return f"{prefix}:{uuid.uuid4().hex[:8]}"


class TestSession:
    """Tests for Session class."""

    def test_create_session(self):
        """Test creating a new session."""
        session = Session(key="test:channel")
        assert session.key == "test:channel"
        assert len(session.messages) == 0
        assert session.created_at is not None
        assert session.updated_at is not None

    def test_add_message(self):
        """Test adding messages to a session."""
        session = Session(key="test:channel")
        session.add_message("user", "Hello!")
        session.add_message("assistant", "Hi there!")

        messages = session.messages
        assert len(messages) == 2
        assert messages[0]["role"] == "user"
        assert messages[0]["content"] == "Hello!"
        assert messages[1]["role"] == "assistant"
        assert messages[1]["content"] == "Hi there!"

    def test_add_message_with_kwargs(self):
        """Test adding a message with extra kwargs."""
        session = Session(key="test:channel")
        session.add_message("user", "Hello!", custom_field="value")

        messages = session.messages
        assert len(messages) == 1
        assert messages[0]["custom_field"] == "value"

    def test_get_history(self):
        """Test getting message history."""
        session = Session(key="test:channel")
        for i in range(10):
            session.add_message("user", f"Message {i}")

        # Get all messages
        history = session.get_history(max_messages=50)
        assert len(history) == 10

        # Get only last 5
        history = session.get_history(max_messages=5)
        assert len(history) == 5
        assert history[0]["content"] == "Message 5"

    def test_get_history_format(self):
        """Test that get_history returns LLM format (only role and content)."""
        session = Session(key="test:channel")
        session.add_message("user", "Hello!", extra_field="ignored")

        history = session.get_history()
        assert len(history) == 1
        assert "role" in history[0]
        assert "content" in history[0]
        # Extra fields should not be in history
        assert "extra_field" not in history[0]
        assert "timestamp" not in history[0]

    def test_clear(self):
        """Test clearing a session."""
        session = Session(key="test:channel")
        session.add_message("user", "Hello!")
        session.add_message("assistant", "Hi!")

        session.clear()

        assert len(session.messages) == 0

    def test_session_with_initial_messages(self):
        """Test creating a session with initial messages."""
        initial_messages = [
            {"role": "user", "content": "Hello", "timestamp": "2024-01-01T00:00:00"},
            {"role": "assistant", "content": "Hi", "timestamp": "2024-01-01T00:00:01"},
        ]
        session = Session(key="test:channel", messages=initial_messages)

        assert len(session.messages) == 2
        assert session.messages[0]["content"] == "Hello"

    def test_metadata(self):
        """Test session metadata."""
        session = Session(key="test:channel", metadata={"key": "value"})

        meta = session.metadata
        assert meta.get("key") == "value"

        # Update metadata
        session.metadata = {"new_key": "new_value"}
        assert session.metadata.get("new_key") == "new_value"


class TestSessionManager:
    """Tests for SessionManager class."""

    @pytest.fixture
    def temp_workspace(self):
        """Create a temporary workspace directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            yield Path(tmpdir)

    @pytest.fixture
    def manager(self, temp_workspace):
        """Create a session manager with temp workspace."""
        return SessionManager(temp_workspace)

    def test_get_or_create_new_session(self, manager):
        """Test getting a new session."""
        key = unique_key()
        session = manager.get_or_create(key)
        assert session.key == key
        assert len(session.messages) == 0

    def test_get_or_create_returns_cached(self, manager):
        """Test that get_or_create returns cached session after save."""
        key = unique_key()
        session1 = manager.get_or_create(key)
        session1.add_message("user", "Hello")
        manager.save(session1)  # Save to update cache

        # Get the same session again
        session2 = manager.get_or_create(key)

        # Should have the message we added (from cache/disk)
        assert len(session2.messages) == 1

    def test_save_and_load(self, manager):
        """Test saving and loading a session."""
        key = unique_key()
        session = manager.get_or_create(key)
        session.add_message("user", "Saved message")
        session.add_message("assistant", "Got it!")
        manager.save(session)

        # Create new manager to clear cache
        new_manager = SessionManager(
            manager.workspace if hasattr(manager, "workspace") else Path.home()
        )
        loaded = new_manager.get_or_create(key)

        assert len(loaded.messages) == 2
        assert loaded.messages[0]["content"] == "Saved message"

    def test_delete(self, manager):
        """Test deleting a session."""
        key = unique_key()
        session = manager.get_or_create(key)
        session.add_message("user", "This will be deleted")
        manager.save(session)

        # Delete it
        result = manager.delete(key)
        assert result is True

        # Getting it should create a fresh one
        session2 = manager.get_or_create(key)
        assert len(session2.messages) == 0

    def test_delete_nonexistent(self, manager):
        """Test deleting a session that doesn't exist."""
        result = manager.delete("test:nonexistent")
        assert result is False

    def test_list_sessions(self, manager):
        """Test listing sessions."""
        # Create some sessions with unique keys
        # Note: key format should be channel:chat_id (no underscores to avoid conversion issues)
        prefix = uuid.uuid4().hex[:8]
        created_keys = []
        for i in range(3):
            key = f"testlist{i}:{prefix}"
            created_keys.append(key)
            session = manager.get_or_create(key)
            session.add_message("user", f"Message {i}")
            manager.save(session)

        sessions = manager.list_sessions()

        # Should have at least 3 sessions
        all_keys = [s["key"] for s in sessions]
        # Check that all our created keys are in the list
        for key in created_keys:
            expected_key = key.replace(":", "_").replace("_", ":")  # After round-trip
            assert expected_key in all_keys, f"Expected {expected_key} in {all_keys}"


class TestPythonFallback:
    """Tests for Python fallback implementation."""

    def test_fallback_import(self):
        """Test that Python fallback modules can be imported."""
        from debot.session._manager_py import Session as PySession
        from debot.session._manager_py import SessionManager as PySessionManager

        assert PySession is not None
        assert PySessionManager is not None
