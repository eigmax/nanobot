"""Tests for the context module (Rust implementation)."""

import tempfile
from pathlib import Path

import pytest

from debot.agent.context import ContextBuilder
from debot.agent.skills import SkillsLoader


class TestContextBuilder:
    """Tests for ContextBuilder class."""

    @pytest.fixture
    def temp_workspace(self):
        """Create a temporary workspace directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            workspace = Path(tmpdir)
            # Create required directories
            (workspace / "memory").mkdir()
            (workspace / "skills").mkdir()
            yield workspace

    @pytest.fixture
    def builder(self, temp_workspace):
        """Create a ContextBuilder with temp workspace."""
        return ContextBuilder(temp_workspace)

    def test_create_context_builder(self, temp_workspace):
        """Test creating a new context builder."""
        builder = ContextBuilder(temp_workspace)
        assert builder.workspace == str(temp_workspace)

    def test_build_system_prompt(self, builder):
        """Test building system prompt."""
        prompt = builder.build_system_prompt()
        assert "debot" in prompt
        assert "workspace" in prompt.lower()

    def test_build_system_prompt_includes_time(self, builder):
        """Test that system prompt includes current time."""
        prompt = builder.build_system_prompt()
        assert "Current Time" in prompt

    def test_build_system_prompt_with_bootstrap_files(self, temp_workspace):
        """Test system prompt includes bootstrap files."""
        # Create a bootstrap file
        (temp_workspace / "SOUL.md").write_text("Be helpful and kind.")

        builder = ContextBuilder(temp_workspace)
        prompt = builder.build_system_prompt()

        assert "SOUL.md" in prompt
        assert "Be helpful and kind" in prompt

    def test_build_system_prompt_with_memory(self, temp_workspace):
        """Test system prompt includes memory context."""
        # Create memory file
        memory_dir = temp_workspace / "memory"
        (memory_dir / "MEMORY.md").write_text("User prefers Python.")

        builder = ContextBuilder(temp_workspace)
        prompt = builder.build_system_prompt()

        assert "Memory" in prompt
        assert "User prefers Python" in prompt

    def test_build_messages(self, builder):
        """Test building message list."""
        history = []
        messages = builder.build_messages(history, "Hello!")

        assert len(messages) == 2  # system + user
        assert messages[0]["role"] == "system"
        assert messages[1]["role"] == "user"
        assert messages[1]["content"] == "Hello!"

    def test_build_messages_with_history(self, builder):
        """Test building messages with conversation history."""
        history = [
            {"role": "user", "content": "Hi"},
            {"role": "assistant", "content": "Hello!"},
        ]
        messages = builder.build_messages(history, "How are you?")

        assert len(messages) == 4  # system + 2 history + user
        assert messages[1]["content"] == "Hi"
        assert messages[2]["content"] == "Hello!"
        assert messages[3]["content"] == "How are you?"

    def test_add_tool_result(self, builder):
        """Test adding tool result to messages."""
        messages = [{"role": "system", "content": "test"}]
        result = builder.add_tool_result(messages, "call-123", "read_file", "file contents")

        assert len(result) == 2
        assert result[1]["role"] == "tool"
        assert result[1]["tool_call_id"] == "call-123"
        assert result[1]["name"] == "read_file"
        assert result[1]["content"] == "file contents"

    def test_add_assistant_message(self, builder):
        """Test adding assistant message."""
        messages = [{"role": "system", "content": "test"}]
        result = builder.add_assistant_message(messages, "Here's the answer")

        assert len(result) == 2
        assert result[1]["role"] == "assistant"
        assert result[1]["content"] == "Here's the answer"

    def test_add_assistant_message_with_tool_calls(self, builder):
        """Test adding assistant message with tool calls."""
        messages = [{"role": "system", "content": "test"}]
        tool_calls = [{"id": "call-123", "type": "function", "function": {"name": "test"}}]
        result = builder.add_assistant_message(messages, "Let me check", tool_calls)

        assert len(result) == 2
        assert result[1]["role"] == "assistant"
        assert "tool_calls" in result[1]


class TestSkillsLoader:
    """Tests for SkillsLoader class."""

    @pytest.fixture
    def temp_workspace(self):
        """Create a temporary workspace directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            workspace = Path(tmpdir)
            (workspace / "skills").mkdir()
            yield workspace

    @pytest.fixture
    def loader(self, temp_workspace):
        """Create a SkillsLoader with temp workspace."""
        return SkillsLoader(temp_workspace)

    def test_create_skills_loader(self, temp_workspace):
        """Test creating a skills loader."""
        loader = SkillsLoader(temp_workspace)
        assert loader is not None

    def test_list_skills_empty(self, loader):
        """Test listing skills when none exist."""
        skills = loader.list_skills()
        assert len(skills) == 0

    def test_list_skills_with_workspace_skill(self, temp_workspace):
        """Test listing skills from workspace."""
        # Create a skill
        skill_dir = temp_workspace / "skills" / "test-skill"
        skill_dir.mkdir()
        (skill_dir / "SKILL.md").write_text("# Test Skill\n\nThis is a test.")

        loader = SkillsLoader(temp_workspace)
        skills = loader.list_skills()

        assert len(skills) == 1
        assert skills[0]["name"] == "test-skill"
        assert skills[0]["source"] == "workspace"

    def test_load_skill(self, temp_workspace):
        """Test loading a skill by name."""
        skill_dir = temp_workspace / "skills" / "my-skill"
        skill_dir.mkdir()
        (skill_dir / "SKILL.md").write_text("# My Skill\n\nDo something.")

        loader = SkillsLoader(temp_workspace)
        content = loader.load_skill("my-skill")

        assert content is not None
        assert "My Skill" in content

    def test_load_skill_not_found(self, loader):
        """Test loading a skill that doesn't exist."""
        content = loader.load_skill("nonexistent")
        assert content is None

    def test_load_skills_for_context(self, temp_workspace):
        """Test loading multiple skills for context."""
        # Create skills
        for name in ["skill-a", "skill-b"]:
            skill_dir = temp_workspace / "skills" / name
            skill_dir.mkdir()
            (skill_dir / "SKILL.md").write_text(f"# {name}\n\nDescription of {name}.")

        loader = SkillsLoader(temp_workspace)
        content = loader.load_skills_for_context(["skill-a", "skill-b"])

        assert "skill-a" in content
        assert "skill-b" in content
        assert "---" in content  # Skills are separated by ---

    def test_build_skills_summary(self, temp_workspace):
        """Test building skills summary."""
        skill_dir = temp_workspace / "skills" / "summary-test"
        skill_dir.mkdir()
        (skill_dir / "SKILL.md").write_text("---\ndescription: A test skill\n---\n# Test")

        loader = SkillsLoader(temp_workspace)
        summary = loader.build_skills_summary()

        assert "<skills>" in summary
        assert "summary-test" in summary
        assert "</skills>" in summary

    def test_get_skill_metadata(self, temp_workspace):
        """Test getting skill metadata from frontmatter."""
        skill_dir = temp_workspace / "skills" / "meta-skill"
        skill_dir.mkdir()
        (skill_dir / "SKILL.md").write_text(
            "---\ndescription: My description\nauthor: Test\n---\n# Content"
        )

        loader = SkillsLoader(temp_workspace)
        meta = loader.get_skill_metadata("meta-skill")

        assert meta is not None
        assert meta.get("description") == "My description"
        assert meta.get("author") == "Test"

    def test_get_skill_metadata_no_frontmatter(self, temp_workspace):
        """Test getting metadata when no frontmatter exists."""
        skill_dir = temp_workspace / "skills" / "no-meta"
        skill_dir.mkdir()
        (skill_dir / "SKILL.md").write_text("# Just content\n\nNo frontmatter here.")

        loader = SkillsLoader(temp_workspace)
        meta = loader.get_skill_metadata("no-meta")

        assert meta is None


class TestPythonFallback:
    """Tests for Python fallback implementations."""

    def test_context_fallback_import(self):
        """Test that Python context fallback can be imported."""
        from debot.agent._context_py import ContextBuilder as PyContextBuilder

        assert PyContextBuilder is not None

    def test_skills_fallback_import(self):
        """Test that Python skills fallback can be imported."""
        from debot.agent._skills_py import SkillsLoader as PySkillsLoader

        assert PySkillsLoader is not None
