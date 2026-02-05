"""Heartbeat service for periodic agent wake-ups."""

try:
    from nanobot_rust import HeartbeatService
except ImportError:
    from nanobot.heartbeat._service_py import HeartbeatService

__all__ = ["HeartbeatService"]
