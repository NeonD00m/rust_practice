# Implementation Plan: `bake-mcp` (Workspace Agent Bridge)

## 1. Overview & Architectural Goals
`bake-mcp` is an independent workspace binary in `rust_practice/` providing a Model Context Protocol (MCP) server over standard I/O (stdio). It exposes tools from `todo` (task tracking) and `palmtree` (Pomodoro timer sessions) to AI coding agents (Claude, Codex, Antigravity, Cursor).

### Architectural Invariants
1. **Preserve Zero-Bloat in Sibling Crates:** `todoCli` and `pomodoro` must NOT absorb any async runtime (`tokio`), schema generators (`schemars`), or networking/RPC dependencies.
2. **Subprocess Isolation vs. Shared Configs:** 
   - `bake-mcp` operates either by executing sibling binaries via `std::process::Command` or reading their persistent state files directly (`my_todo.json`, `~/.config/palmtree/config.json`).
3. **Strict Stdio Hygiene:** Since MCP communicates via JSON-RPC across stdin/stdout, all logging, tracing, and diagnostics must strictly route to `stderr`.

---

## 2. Workspace Layout

```text
rust_practice/
├── Cargo.toml                  # Workspace root (virtual manifest)
├── todoCli/                    # Synchronous, ultra-fast task CLI
│   ├── src/main.rs
│   └── Cargo.toml
├── pomodoro/                   # Terminal Pomodoro timer
│   ├── src/main.rs
│   └── Cargo.toml
└── bake-mcp/               # NEW: MCP Server Daemon
    ├── Cargo.toml
    └── src/
        ├── main.rs             # CLI entrypoint & tracing setup
        ├── transport.rs        # Clean stdio reader/writer with probe filtering
        ├── server.rs           # rmcp router & ServerHandler implementation
        ├── tools/
        │   ├── mod.rs
        │   ├── todo.rs         # Todo list inspection & manipulation
        │   └── palmtree.rs     # Timer management & config queries
        └── adapters/
            ├── todo_adapter.rs # Subprocess execution / JSON reader for todo
            └── palm_adapter.rs # Config reader / runner for palmtree
```

---

## 3. Dependency Specification (`bake-mcp/Cargo.toml`)

Adopt the battle-tested MCP stack from Quizzy:

```toml
[package]
name = "bake-mcp"
version = "0.1.0"
edition = "2024"

[dependencies]
# MCP SDK & Async Runtime
rmcp = { version = "2.2.0", features = ["transport-io", "uuid"] }
tokio = { version = "1.48.0", features = ["rt-multi-thread", "macros", "io-util", "sync", "process"] }
schemars = "1.2.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Stderr-only diagnostic logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
dirs = "6.0"
anyhow = "1.0"
```

---

## 4. Exposed MCP Tools Specification

### A. Todo Tools (`tools/todo.rs`)
| Tool Name | Parameters | Description | Backend Action |
| :--- | :--- | :--- | :--- |
| `todo_list` | `{ incomplete_only?: bool, page?: u32 }` | List current tasks with numbers, status, and tags. | Reads `my_todo.json` or runs `todo l`. |
| `todo_add` | `{ text: String, tags?: Vec<String> }` | Create a new task with optional tags. | Executes `todo n <text>` or appends to JSON. |
| `todo_complete` | `{ task_ids: Vec<usize> }` | Mark one or more tasks as completed. | Executes `todo c <id...>`. |
| `todo_search` | `{ query: String, search_text_only?: bool }` | Search tasks by keyword or tags. | Executes `todo s <query>`. |

### B. Palmtree Tools (`tools/palmtree.rs`)
| Tool Name | Parameters | Description | Backend Action |
| :--- | :--- | :--- | :--- |
| `palm_get_presets` | `{}` | Retrieve productivity ratios (Pomodoro, DeskTime, Flow, etc.). | Inspects palmtree presets/config. |
| `palm_get_config` | `{}` | View configured work/break durations and sound preferences. | Reads `~/.config/palmtree/config.json`. |
| `palm_prepare_task` | `{ task_id: usize }` | Resolves a task name from `todo` formatted specifically for piping into palmtree. | Runs `todo print <id>`. |

---

## 5. Implementation Phases

### Phase 1: Workspace Initialization
1. Update root `rust_practice/Cargo.toml` to declare `members = ["todoCli", "pomodoro", "bake-mcp"]`.
2. Scaffold `bake-mcp` with dependencies matching the schema above.

### Phase 2: Stdio Transport & Protocol Plumbing
1. Implement `transport.rs` porting the `create_stdio_transport()` buffer filter from Quizzy (`src/mcp/server.rs`).
2. Intercept `server/discover` probes to respond with clean JSON-RPC Method Not Found errors instead of crashing the parser.
3. Configure `tracing_subscriber` targeting `std::io::stderr` strictly with ANSI colors disabled.

### Phase 3: Adapters & Tool Implementations
1. Implement `todo_adapter.rs`:
   - Locate `my_todo.json` in the current working directory or fall back to spawning the `todo` binary found in `PATH` / cargo target directory.
2. Implement `palm_adapter.rs`:
   - Parse `~/.config/palmtree/config.json` for timer metrics.
3. Register tools using `rmcp` macros (`#[tool_router]`, `#[tool]`, `nullable_to_anyof`).

### Phase 4: Agent Testing & Configuration
1. Register server in local agent configs:
   ```json
   {
     "mcpServers": {
       "practice": {
         "command": "cargo",
         "args": ["run", "--release", "-p", "bake-mcp"]
       }
     }
   }
   ```
2. Test end-to-end task flows from Codex / Claude:
   - *"List my open tasks, find anything related to refactoring, and prepare it for my next palmtree timer."*
