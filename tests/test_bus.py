"""Tests for the message bus module."""

import asyncio
import pytest

from debot.bus import MessageBus, InboundMessage, OutboundMessage


class TestInboundMessage:
    """Tests for InboundMessage."""

    def test_basic_creation(self):
        """Test creating an InboundMessage with required fields."""
        msg = InboundMessage(
            channel="telegram", sender_id="user123", chat_id="chat456", content="Hello World!"
        )
        assert msg.channel == "telegram"
        assert msg.sender_id == "user123"
        assert msg.chat_id == "chat456"
        assert msg.content == "Hello World!"

    def test_session_key(self):
        """Test session_key property."""
        msg = InboundMessage(channel="whatsapp", sender_id="user1", chat_id="chat1", content="test")
        assert msg.session_key == "whatsapp:chat1"

    def test_timestamp_auto_generated(self):
        """Test that timestamp is automatically generated."""
        msg = InboundMessage(channel="telegram", sender_id="user1", chat_id="chat1", content="test")
        assert msg.timestamp > 0

    def test_custom_timestamp(self):
        """Test creating with a custom timestamp."""
        msg = InboundMessage(
            channel="telegram",
            sender_id="user1",
            chat_id="chat1",
            content="test",
            timestamp=1234567890.0,
        )
        assert msg.timestamp == 1234567890.0

    def test_media_list(self):
        """Test media field."""
        msg = InboundMessage(
            channel="telegram",
            sender_id="user1",
            chat_id="chat1",
            content="test",
            media=["https://example.com/image.png", "https://example.com/file.pdf"],
        )
        assert len(msg.media) == 2
        assert msg.media[0] == "https://example.com/image.png"

    def test_metadata_dict(self):
        """Test metadata field."""
        msg = InboundMessage(
            channel="telegram",
            sender_id="user1",
            chat_id="chat1",
            content="test",
            metadata={"key1": "value1", "count": 42},
        )
        assert msg.metadata["key1"] == "value1"
        assert msg.metadata["count"] == 42

    def test_mutable_fields(self):
        """Test that fields can be modified."""
        msg = InboundMessage(
            channel="telegram", sender_id="user1", chat_id="chat1", content="original"
        )
        msg.content = "modified"
        msg.channel = "whatsapp"
        assert msg.content == "modified"
        assert msg.channel == "whatsapp"


class TestOutboundMessage:
    """Tests for OutboundMessage."""

    def test_basic_creation(self):
        """Test creating an OutboundMessage with required fields."""
        msg = OutboundMessage(channel="telegram", chat_id="chat456", content="Response!")
        assert msg.channel == "telegram"
        assert msg.chat_id == "chat456"
        assert msg.content == "Response!"
        assert msg.reply_to is None

    def test_reply_to(self):
        """Test reply_to field."""
        msg = OutboundMessage(
            channel="telegram", chat_id="chat456", content="Response!", reply_to="msg123"
        )
        assert msg.reply_to == "msg123"

    def test_media_list(self):
        """Test media field."""
        msg = OutboundMessage(
            channel="telegram",
            chat_id="chat456",
            content="test",
            media=["https://example.com/image.png"],
        )
        assert len(msg.media) == 1

    def test_metadata_dict(self):
        """Test metadata field."""
        msg = OutboundMessage(
            channel="telegram", chat_id="chat456", content="test", metadata={"sent": True}
        )
        assert msg.metadata["sent"] is True


class TestMessageBus:
    """Tests for MessageBus."""

    @pytest.fixture
    def bus(self):
        """Create a fresh MessageBus for each test."""
        return MessageBus()

    @pytest.mark.asyncio
    async def test_publish_consume_inbound(self, bus):
        """Test basic inbound publish/consume cycle."""
        msg = InboundMessage(channel="test", sender_id="user1", chat_id="chat1", content="Hello")
        await bus.publish_inbound(msg)
        assert bus.inbound_size == 1

        result = await asyncio.wait_for(bus.consume_inbound(), timeout=1.0)
        assert result.content == "Hello"
        assert bus.inbound_size == 0

    @pytest.mark.asyncio
    async def test_publish_consume_outbound(self, bus):
        """Test basic outbound publish/consume cycle."""
        msg = OutboundMessage(channel="test", chat_id="chat1", content="Response")
        await bus.publish_outbound(msg)
        assert bus.outbound_size == 1

        result = await asyncio.wait_for(bus.consume_outbound(), timeout=1.0)
        assert result.content == "Response"
        assert bus.outbound_size == 0

    @pytest.mark.asyncio
    async def test_multiple_messages_fifo(self, bus):
        """Test that messages are consumed in FIFO order."""
        for i in range(5):
            msg = InboundMessage(
                channel="test", sender_id="user1", chat_id="chat1", content=f"Message {i}"
            )
            await bus.publish_inbound(msg)

        assert bus.inbound_size == 5

        for i in range(5):
            result = await asyncio.wait_for(bus.consume_inbound(), timeout=1.0)
            assert result.content == f"Message {i}"

        assert bus.inbound_size == 0

    @pytest.mark.asyncio
    async def test_consume_blocks_until_message(self, bus):
        """Test that consume blocks until a message is available."""

        async def publish_after_delay():
            await asyncio.sleep(0.1)
            msg = InboundMessage(
                channel="test", sender_id="user1", chat_id="chat1", content="Delayed"
            )
            await bus.publish_inbound(msg)

        asyncio.create_task(publish_after_delay())

        result = await asyncio.wait_for(bus.consume_inbound(), timeout=1.0)
        assert result.content == "Delayed"

    @pytest.mark.asyncio
    async def test_consume_timeout(self, bus):
        """Test that consume times out when no message is available."""
        with pytest.raises(asyncio.TimeoutError):
            await asyncio.wait_for(bus.consume_inbound(), timeout=0.1)

    def test_stop(self, bus):
        """Test stop method doesn't raise."""
        bus.stop()  # Should not raise

    def test_initial_sizes_are_zero(self, bus):
        """Test that initial queue sizes are zero."""
        assert bus.inbound_size == 0
        assert bus.outbound_size == 0

    @pytest.mark.asyncio
    async def test_message_preserves_all_fields(self, bus):
        """Test that all message fields are preserved through the bus."""
        original = InboundMessage(
            channel="telegram",
            sender_id="user123",
            chat_id="chat456",
            content="Full message",
            media=["https://example.com/img.png"],
            metadata={"key": "value", "num": 42},
        )
        await bus.publish_inbound(original)
        result = await asyncio.wait_for(bus.consume_inbound(), timeout=1.0)

        assert result.channel == original.channel
        assert result.sender_id == original.sender_id
        assert result.chat_id == original.chat_id
        assert result.content == original.content
        assert result.media == original.media
        assert result.metadata["key"] == "value"
        assert result.metadata["num"] == 42


class TestPythonFallback:
    """Tests specifically for the Python fallback implementation."""

    def test_fallback_import(self):
        """Test that the Python fallback modules exist."""
        from debot.bus._events_py import InboundMessage as PyInbound
        from debot.bus._events_py import OutboundMessage as PyOutbound
        from debot.bus._queue_py import MessageBus as PyBus

        # Verify they work
        msg = PyInbound(channel="test", sender_id="user1", chat_id="chat1", content="test")
        assert msg.content == "test"
