# Installing & Configuring RSICraft

**RSICraft** compiles to a single standalone Rust binary (`rsicraft` / `rsicraft.exe`) with zero Python virtual environment overhead and zero background daemon ports required for MCP operation.

---

## Prerequisites

- **Rust Toolchain:** Rust `1.85+` (Edition 2021) via [rustup.rs](https://rustup.rs) for building from source.
- **Optional (Live LLM Synthesis on RTX 4090):** `llama-server` ([llama.cpp](https://github.com/ggerganov/llama.cpp)) or Ollama serving an OpenAI-compatible `/v1/chat/completions` endpoint.
- **Optional (Live Sandbox Evaluation):**
  - **Local Mode (`--sandbox-mode local`):** Python `3.10+` (`python` or `uv` Python 3.12) and/or `rustc` for native Rust candidate compilation.
  - **Container Mode (`--sandbox-mode docker`):** Docker Desktop / rootless Docker Engine.
  - **Offline Mode (`--provider demo --sandbox-mode demo`):** Zero external dependencies.

---

## 1. Build from Source

```powershell
git clone https://github.com/sandraschi/rsicraft.git
cd rsicraft
cargo build --release -p rsicraft-mcp
```

The release binary is produced at:
- **Windows:** `target\release\rsicraft.exe`
- **Linux / macOS:** `target/release/rsicraft`

---

## 2. Connect to MCP Clients (`2024-11-05` Stdio)

### Claude Desktop / Antigravity / Windsurf (`claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "rsicraft": {
      "command": "D:\\Dev\\repos\\rsicraft\\target\\release\\rsicraft.exe",
      "args": [
        "--db",
        "C:\\Users\\sandr\\AppData\\Local\\rsicraft\\rsicraft.db",
        "--llama-url",
        "http://127.0.0.1:8081",
        "mcp"
      ]
    }
  }
}
```

### Cursor (`.cursor/mcp.json`)

```json
{
  "mcpServers": {
    "rsicraft": {
      "command": "rsicraft",
      "args": ["mcp"]
    }
  }
}
```

---

## 3. Configure Local RTX 4090 Inference (`llama-server`)

On **Goliath** (NVIDIA GeForce RTX 4090 24 GB VRAM), launch `llama-server` with full GPU layer offloading and `q4_0` KV cache quantization:

```powershell
llama-server `
  --model D:\Models\frontis-ma1-30b-a3b-q4_k_m.gguf `
  --alias frontis `
  --ctx-size 16384 `
  --n-gpu-layers 99 `
  --cache-type-k q4_0 `
  --cache-type-v q4_0 `
  --flash-attn `
  --parallel 2 `
  --port 8081
```

> **Tip:** If Ollama is already running an active `llama-server.exe` child process on a dynamic port (inspect via `Get-CimInstance Win32_Process -Filter "Name = 'llama-server.exe'"`), pass `--llama-url http://127.0.0.1:<port>` directly to `rsicraft` to avoid Ollama `:11434` model-swap reloads.

---

## 4. Build the Air-Gapped Docker Evaluator (Optional)

To use `--sandbox-mode docker`:

```powershell
docker build -t rsicraft-evaluator:latest -f sandboxes/Dockerfile.evaluator sandboxes/
```

If Docker Desktop is not running, pass `--sandbox-mode local` to evaluate inside an isolated local Python 3.12 temporary directory with wall-clock timeout reaping.

---

## 5. Verification Steps

Run these three commands to confirm your installation is healthy:

```powershell
# 1. Inspect hardware profile and VRAM budget
rsicraft hardware

# 2. Probe the active llama-server endpoint
rsicraft probe

# 3. Seed a temporary database and inspect the top-K Pareto frontier
rsicraft --db verify.db demo-seed
rsicraft --db verify.db frontier --task-id opt-synth

# 4. Read the 10-tab manual
rsicraft help-tabs --tab start
```

 Or ask your MCP client:
> *"Call `rsi_status`, `rsi_hardware_profile`, and `rsi_help` with `tab: 'start'` to verify RSICraft is online."*
