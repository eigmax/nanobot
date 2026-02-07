# Telegram `/compact` Command

## Overview

The `/compact` command allows you to trigger session history compaction directly from Telegram chat. This is useful when conversations become lengthy and you want to reduce the context window without losing important information.

## Usage

### Basic Usage

Simply send:
```
/compact
```

This compacts the conversation history, keeping the last 50 messages (default) and summarizing older messages into a compact entry.

### With Keep-Last Parameter

To keep a different number of recent messages:
```
/compact 30
/compact 100
```

### Silent vs Verbose Mode

By default, compaction runs in **silent mode** (no logs in the console).

For verbose output with detailed logs:
```
/compact --verbose
/compact 30 --verbose
```

To explicitly run in silent mode:
```
/compact --silent
/compact 50 --silent
```

## Examples

### Scenario 1: Quick Compact with Defaults
```
User: /compact
Bot: 🧹 Compaction complete!
  Compacted: 152 messages
  Kept: 50 recent messages
  Total compactions: 1
  Messages compacted (total): 152
```

### Scenario 2: Keep More Messages
```
User: /compact 80
Bot: 🧹 Compaction complete!
  Compacted: 89 messages
  Kept: 80 recent messages
  Total compactions: 2
  Messages compacted (total): 241
```

### Scenario 3: Short History (No Compaction Needed)
```
User: /compact
Bot: ℹ️ No messages to compact (history is short enough)
```

## How It Works

1. **Detects Command**: When you send `/compact`, Telegram channel intercepts it
2. **Parses Arguments**: Extracts keep-last count and silent/verbose flag
3. **Compacts Session**: Summarizes old messages into a single system entry
4. **Persists Data**: Saves compaction telemetry (count, timestamp, total)
5. **Reports Result**: Sends formatted status message back to you

## Configuration

Compaction behavior can be configured globally or per-model:

### Global Config
```bash
debot config compaction --show
debot config compaction --enabled --keep-last 50 --trigger-ratio 0.9
```

### Per-Model Config
```bash
debot config compaction-model "anthropic/claude-opus-4-5" --keep-last 40
debot config compaction-model "openai/gpt-4" --trigger-ratio 0.85
```

### Telegram Auto-Compaction

Auto-compaction triggers when context window usage exceeds the configured threshold (default 90% of max_tokens). This happens automatically during regular chat and doesn't require `/compact` command.

The `/compact` command provides **manual on-demand compaction** independent of the auto-compaction trigger.

## Telemetry

Each session maintains compaction telemetry in its metadata:

```json
{
  "compactions": {
    "total": 2,              // Number of times compacted
    "count": 241,            // Total messages compacted
    "last_at": "2026-02-06T10:56:54.781010",  // Last compaction time
    "messages_compacted": 241    // Cumulative count
  }
}
```

View session statistics programmatically:
```python
from debot.session import SessionManager
sm = SessionManager(workspace_path)
session = sm.get_or_create("telegram:12345")
print(session.metadata.get('compactions'))
```

## Limitations

- `/compact` only affects the current chat session
- Cannot compact other users' sessions (security)
- Minimum keep_last is 0; if you provide a very small number, recent messages are preserved anyway
- Command is rate-limited by Telegram's message handling (no more than 1 per message handler invocation)

## Troubleshooting

### "Compaction failed: session not found"
The session doesn't exist yet. Send a regular message first to create the session, then try `/compact`.

### "No messages to compact"
Your conversation is short (fewer messages than keep_last parameter). The compaction has nothing to do.

### Bot doesn't respond to `/compact`
- Ensure bot is running: `debot gateway`
- Check Telegram token is configured: `debot channels status`
- Verify you're in an allowed chat (if allowlist is set in config)
