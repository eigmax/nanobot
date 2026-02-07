<div align="center">
  <img src="debot_logo.png" alt="debot" width="500">
  <h1>debot: Ultra-Lightweight Personal AI Assistant</h1>
  <p>
    <a href="https://pypi.org/project/debot-ai/"><img src="https://img.shields.io/pypi/v/debot-ai" alt="PyPI"></a>
    <a href="https://pepy.tech/project/debot-ai"><img src="https://static.pepy.tech/badge/debot-ai" alt="Downloads"></a>
    <img src="https://img.shields.io/badge/python-≥3.11-blue" alt="Python">
    <img src="https://img.shields.io/badge/license-MIT-green" alt="License">
    <a href="./COMMUNICATION.md"><img src="https://img.shields.io/badge/Feishu-Group-E9DBFC?style=flat&logo=feishu&logoColor=white" alt="Feishu"></a>
    <a href="./COMMUNICATION.md"><img src="https://img.shields.io/badge/WeChat-Group-C5EAB4?style=flat&logo=wechat&logoColor=white" alt="WeChat"></a>
    <a href="https://discord.gg/MnCvHqpUGB"><img src="https://img.shields.io/badge/Discord-Community-5865F2?style=flat&logo=discord&logoColor=white" alt="Discord"></a>
  </p>
</div>

🐈 **debot** is an **ultra-lightweight** personal AI assistant inspired by [Clawdbot](https://github.com/openclaw/openclaw) 

⚡️ Delivers core agent functionality in just **~4,000** lines of code — **99% smaller** than Clawdbot's 430k+ lines.

## Key Features of debot:

🪶 **Ultra-Lightweight**: Just ~4,000 lines of code — 99% smaller than Clawdbot - core functionality.

🔬 **Research-Ready**: Clean, readable code that's easy to understand, modify, and extend for research.

⚡️ **Lightning Fast**: Minimal footprint means faster startup, lower resource usage, and quicker iterations.


### CI / Docker notes for building the Rust extension

When building the Rust Python extension inside CI or containers on newer Python versions (for example Python 3.14), set the following environment variable so PyO3 uses the stable ABI forward-compatibility:

```bash
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
```

If you need to specify a particular Python executable for maturin builds, set `PYO3_PYTHON` to the interpreter path.
💎 **Easy-to-Use**: One-click to depoly and you're ready to go.

## 🏗️ Architecture

<p align="center">
  <img src="debot_arch.png" alt="debot architecture" width="800">
</p>

## ✨ Features

<table align="center">
  <tr align="center">
    <th><p align="center">📈 24/7 Real-Time Market Analysis</p></th>
    <th><p align="center">🚀 Full-Stack Software Engineer</p></th>
    <th><p align="center">📅 Smart Daily Routine Manager</p></th>
    <th><p align="center">📚 Personal Knowledge Assistant</p></th>
  </tr>
  <tr>
    <td align="center"><p align="center"><img src="case/search.gif" width="180" height="400"></p></td>
    <td align="center"><p align="center"><img src="case/code.gif" width="180" height="400"></p></td>
    <td align="center"><p align="center"><img src="case/scedule.gif" width="180" height="400"></p></td>
    <td align="center"><p align="center"><img src="case/memory.gif" width="180" height="400"></p></td>
  </tr>
  <tr>
    <td align="center">Discovery • Insights • Trends</td>
    <td align="center">Develop • Deploy • Scale</td>
    <td align="center">Schedule • Automate • Organize</td>
    <td align="center">Learn • Memory • Reasoning</td>
  </tr>
</table>

## 📦 Install

**Install from source** (latest features, recommended for development)

```bash
git clone https://github.com/BotMesh/debot.git
cd debot
pip install -e .
```

**Install with [uv](https://github.com/astral-sh/uv)** (stable, fast)

```bash
uv tool install debot-ai
```

**Install from PyPI** (stable)

```bash
pip install debot-ai
```

## 🚀 Quick Start

> [!TIP]
> Set your API key in `~/.debot/config.json`.
> Get API keys: [OpenRouter](https://openrouter.ai/keys) (LLM) · [Brave Search](https://brave.com/search/api/) (optional, for web search)
> You can also change the model to `minimax/minimax-m2` for lower cost.

**1. Initialize**

```bash
debot onboard
```

**2. Configure** (`~/.debot/config.json`)

```json
{
  "providers": {
    "openrouter": {
      "apiKey": "sk-or-v1-xxx"
    }
  },
  "agents": {
    "defaults": {
      "model": "anthropic/claude-opus-4-5"
    }
  },
  "webSearch": {
    "apiKey": "BSA-xxx"
  }
}
```


**3. Chat**

```bash
debot agent -m "What is 2+2?"
```

That's it! You have a working AI assistant in 2 minutes.

## 🖥️ Local Models (vLLM)

Run debot with your own local models using vLLM or any OpenAI-compatible server.

**1. Start your vLLM server**

```bash
vllm serve meta-llama/Llama-3.1-8B-Instruct --port 8000
```

**2. Configure** (`~/.debot/config.json`)

```json
{
  "providers": {
    "vllm": {
      "apiKey": "dummy",
      "apiBase": "http://localhost:8000/v1"
    }
  },
  "agents": {
    "defaults": {
      "model": "meta-llama/Llama-3.1-8B-Instruct"
    }
  }
}
```

**3. Chat**

```bash
debot agent -m "Hello from my local LLM!"
```

## 💾 Session Compaction

debot automatically compacts long conversations to keep context windows efficient. When a conversation exceeds ~90% of the model's context window, old messages are summarized into a single "compaction" entry.

**Features:**
- ✅ **Automatic** — Triggered silently when context limit approached
- ✅ **Manual** — Use `/compact` command in Telegram or CLI
- ✅ **Configurable** — Tune per-model or globally
- ✅ **Tracked** — View compaction stats in session metadata

**Usage:**

```bash
# Manual compaction via CLI
debot sessions compact telegram:12345 --keep-last 50

# View/configure compaction settings
debot config compaction --show
debot config compaction --keep-last 30 --trigger-ratio 0.85

# Per-model settings
debot config compaction-model "anthropic/claude-opus-4-5" --keep-last 40
```

**Telegram:**

```
/compact              # Use default keep-last=50
/compact 30           # Keep last 30 messages
/compact 30 --verbose # Show detailed results
```

## 🧠 Long-term memory

debot stores persistent memory under your workspace at `memory/` (by default your workspace is `~/.debot/workspace`). The memory system supports:

- `MEMORY.md` — long-term notes you want the agent to remember.
- `YYYY-MM-DD.md` — daily notes.
- `.index.json` — a simple local semantic index (auto-generated).

How it works
- The Rust extension (or the Python fallback) exposes `MemoryStore.build_index()` and `MemoryStore.search(query, max_results, min_score)` to build a local vector index and search it.
- If `OPENAI_API_KEY` or `OPENROUTER_API_KEY` is set, debot will attempt to use the remote embeddings API and fall back to a deterministic local embedding when not available.

Quick enable & usage

1. Build and install the Rust extension (in development environments with Python ≧ 3.14 you may need to set `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1`):

```bash
source .venv/bin/activate
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
export PYO3_PYTHON=/opt/homebrew/bin/python3.14
cd rust
maturin build --release -m Cargo.toml
cd ..
pip install rust/target/wheels/*.whl
pip install -e .
```

2. Optionally provide an embeddings key (recommended for better results):

```bash
export OPENAI_API_KEY="sk-..."
# or
export OPENROUTER_API_KEY="or-..."
```

3. Build index and search (Python example):

```python
from pathlib import Path
from debot.agent.memory import search_memory, MemoryStore
# Build index explicitly (if you've updated memory files)
store = MemoryStore(ws)
store.build_index()

# Search
results = search_memory(ws, "when did I last deploy?", max_results=5)
for r in results:
  print(r["score"], r["path"])
  print(r["snippet"][:200])
  print("---")
```

Notes
- If the `.index.json` file is missing, `search_memory()` will attempt to call `build_index()` automatically.
- The local deterministic embedding is SHA256-based and works offline but yields lower-quality semantic matches than remote embeddings.


> [!TIP]
> The `apiKey` can be any non-empty string for local servers that don't require authentication.

## 💬 Chat Apps

Talk to your debot through Telegram or WhatsApp — anytime, anywhere.

| Channel | Setup |
|---------|-------|
| **Telegram** | Easy (just a token) |
| **WhatsApp** | Medium (scan QR) |

<details>
<summary><b>Telegram</b> (Recommended)</summary>

**1. Create a bot**
- Open Telegram, search `@BotFather`
- Send `/newbot`, follow prompts
- Copy the token

**2. Configure**

```json
{
  "channels": {
    "telegram": {
      "enabled": true,
      "token": "YOUR_BOT_TOKEN",
      "allowFrom": ["YOUR_USER_ID"]
    }
  }
}
```

> Get your user ID from `@userinfobot` on Telegram.

**3. Run**

```bash
debot gateway
```

</details>

<details>
<summary><b>WhatsApp</b></summary>

Requires **Node.js ≥18**.

**1. Link device**

```bash
debot channels login
# Scan QR with WhatsApp → Settings → Linked Devices
```

**2. Configure**

```json
{
  "channels": {
    "whatsapp": {
      "enabled": true,
      "allowFrom": ["+1234567890"]
    }
  }
}
```

**3. Run** (two terminals)

```bash
# Terminal 1
debot channels login

# Terminal 2
debot gateway
```

</details>

## ⚙️ Configuration

Config file: `~/.debot/config.json`

### Providers

> [!NOTE]
> Groq provides free voice transcription via Whisper. If configured, Telegram voice messages will be automatically transcribed.

| Provider | Purpose | Get API Key |
|----------|---------|-------------|
| `openrouter` | LLM (recommended, access to all models) | [openrouter.ai](https://openrouter.ai) |
| `anthropic` | LLM (Claude direct) | [console.anthropic.com](https://console.anthropic.com) |
| `openai` | LLM (GPT direct) | [platform.openai.com](https://platform.openai.com) |
| `groq` | LLM + **Voice transcription** (Whisper) | [console.groq.com](https://console.groq.com) |
| `gemini` | LLM (Gemini direct) | [aistudio.google.com](https://aistudio.google.com) |


<details>
<summary><b>Full config example</b></summary>

```json
{
  "agents": {
    "defaults": {
      "model": "anthropic/claude-opus-4-5"
    }
  },
  "providers": {
    "openrouter": {
      "apiKey": "sk-or-v1-xxx"
    },
    "groq": {
      "apiKey": "gsk_xxx"
    }
  },
  "channels": {
    "telegram": {
      "enabled": true,
      "token": "123456:ABC...",
      "allowFrom": ["123456789"]
    },
    "whatsapp": {
      "enabled": false
    }
  },
  "tools": {
    "web": {
      "search": {
        "apiKey": "BSA..."
      }
    }
  }
}
```

</details>

## CLI Reference

| Command | Description |
|---------|-------------|
| `debot onboard` | Initialize config & workspace |
| `debot agent -m "..."` | Chat with the agent |
| `debot agent` | Interactive chat mode |
| `debot gateway` | Start the gateway |
| `debot status` | Show status |
| `debot channels login` | Link WhatsApp (scan QR) |
| `debot channels status` | Show channel status |
| `debot sessions compact <key>` | Manually compact a session |
| `debot config compaction` | View/configure compaction settings |
| `debot config compaction-model <model>` | Set per-model compaction settings |

<details>
<summary><b>Scheduled Tasks (Cron)</b></summary>

```bash
# Add a job
debot cron add --name "daily" --message "Good morning!" --cron "0 9 * * *"
debot cron add --name "hourly" --message "Check status" --every 3600

# List jobs
debot cron list

# Remove a job
debot cron remove <job_id>
```

</details>

## 🐳 Docker

> [!TIP]
> The `-v ~/.debot:/root/.debot` flag mounts your local config directory into the container, so your config and workspace persist across container restarts.

### Build & Run Locally

Build and run debot in a container:

```bash
# Build the image
docker build -t debot .

# Initialize config (first time only)
docker run -v ~/.debot:/root/.debot --rm debot onboard

# Edit config on host to add API keys
vim ~/.debot/config.json

# Run gateway (connects to Telegram/WhatsApp)
docker run -v ~/.debot:/root/.debot -p 18790:18790 debot gateway

# Or run a single command
docker run -v ~/.debot:/root/.debot --rm debot agent -m "Hello!"
docker run -v ~/.debot:/root/.debot --rm debot status
```

### 📦 Pull from GitHub Container Registry

Pre-built images are automatically published to GitHub Container Registry:

```bash
# Pull latest image
docker pull ghcr.io/BotMesh/debot:latest

# Run with pulled image
docker run -v ~/.debot:/root/.debot -p 18790:18790 ghcr.io/BotMesh/debot:latest gateway

# Pull specific version
docker pull ghcr.io/BotMesh/debot:v1.0.0
```

**Available Tags:**
- `latest` — Latest main branch
- `main` — Main branch  
- `v1.0.0` — Release versions
- `main-<short-sha>` — Specific commits

For more info, see [Container Publishing Guide](./.github/CONTAINER_PUBLISHING.md)


## 🤝 Contribute & Roadmap

PRs welcome! The codebase is intentionally small and readable. 🤗

**Roadmap** — Pick an item and [open a PR](https://github.com/BotMesh/debot/pulls)!

- [x] **Voice Transcription** — Support for Groq Whisper (Issue #13)
- [ ] **Multi-modal** — See and hear (images, voice, video)
- [x] **Long-term memory** — Never forget important context
- [ ] **Better reasoning** — Multi-step planning and reflection
- [ ] **More integrations** — Discord, Slack, email, calendar
- [ ] **Self-improvement** — Learn from feedback and mistakes

<p align="center">
  <sub>debot is for educational, research, and technical exchange purposes only</sub>
</p>
