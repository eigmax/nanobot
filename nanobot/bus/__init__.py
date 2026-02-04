"""Message bus module - Rust implementation with Python fallback."""

try:
    from nanobot_rust import InboundMessage, MessageBus, OutboundMessage
except ImportError:
    # Fallback to pure Python if Rust extension not available
    from nanobot.bus._events_py import InboundMessage, OutboundMessage
    from nanobot.bus._queue_py import MessageBus

__all__ = ["MessageBus", "InboundMessage", "OutboundMessage"]
