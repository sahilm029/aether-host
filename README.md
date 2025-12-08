# AETHER: Local-First AI Agent Runtime

![Rust](https://img.shields.io/badge/rust-1.81%2B-orange)
![Tokio](https://img.shields.io/badge/runtime-tokio-blue)
![License](https://img.shields.io/badge/license-MIT-green)

**AETHER** is a high-performance, asynchronous infrastructure runtime for AI Agents. Built in Rust to run efficiently on resource-constrained hardware (e.g., i3 laptops), it implements a capability-based security model (CapSec) to safely execute tools invoked by Large Language Models.

Unlike simple Python wrappers, AETHER is a compiled, multi-threaded host that manages process lifecycles, standardizes Inter-Process Communication (IPC) via the **Model Context Protocol (MCP)**, and provides a TUI dashboard for real-time monitoring.

## ⚡ Core Architecture

* **Runtime:** Rust (2021) + Tokio (Async Event Loop).
* **Protocol:** Custom JSON-RPC 2.0 implementation over Stdio.
* **Security:** Middleware Interceptor enforcing strict `allow/deny` policies via `permissions.json`.
* **Interface:** `ratatui` (Terminal User Interface) with concurrent state management.
* **Intelligence:** Integration with Groq (Llama-3-70b) for neuro-symbolic execution.

## 🚀 Features

* **Async Process Management:** Spawns and supervises child processes (tools) without blocking the main thread.
* **The "Gatekeeper":** A middleware layer that intercepts every LLM tool call. If a tool isn't whitelisted in the config, execution is blocked immediately.
* **Real-Time Dashboard:** A split-screen TUI showing the chat stream on the left and the raw system logs (JSON payloads, security checks) on the right.
* **Zero-Overhead abstractions:** Designed to run with <50MB RAM footprint.

## 🛠️ Usage

### Prerequisites
* Rust Toolchain (`cargo`)
* A Groq API Key (Free tier supported)

### Setup

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/sahilm029/aether-host.git
    cd aether-host
    ```

2.  **Configure Environment:**
    Create a `.env` file in the root directory:
    ```env
    GROQ_API_KEY=gsk_your_key_here...
    ```

3.  **Define Security Rules:**
    Edit `permissions.json` to control what the AI is allowed to do:
    ```json
    {
      "global_policy": "deny",
      "rules": {
        "calculate_sum": "allow",
        "delete_system32": "deny"
      }
    }
    ```

4.  **Compile & Run:**
    ```bash
    # Build the host and the mock tool
    cargo build --bin mock_tool
    cargo run
    ```

## 🧠 System Design (The "ReAct" Loop)

1.  **Input:** User types a command in the TUI.
2.  **Reasoning:** The Host sends the input + Tool Definitions (JSON Schema) to Llama-3 (Cloud).
3.  **Decision:** Llama-3 replies with a `tool_call` request (e.g., "Run Calculator").
4.  **Security Check:** The Host checks `permissions.json`.
    * *If Denied:* Returns a security error to the history.
    * *If Allowed:* Spawns the subprocess, pipes arguments, and captures stdout.
5.  **Result:** The output is fed back to Llama-3 for the final summary.

## 🔮 Roadmap (Codex Engine)

* [o] Phase 1: Async Runtime & Process Spawning
* [o] Phase 2: MCP Protocol Handshake
* [o] Phase 3: Capability-Based Security
* [o] Phase 4: LLM Integration (Groq)
* [o] Phase 5: TUI Dashboard
* [ ] **Phase 6:** Persistent Memory (SQLite)
* [ ] **Phase 7:** AST Parsing (Tree-Sitter) for Code Safety
* [ ] **Phase 8:** Symbolic Execution (Haybale) for Logic Verification

## 📄 License
MIT