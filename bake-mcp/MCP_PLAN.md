# Implementation Plan: `bake-mcp` (Bake Productivity Suite Agent Bridge)

## 1. Overview & Architectural Goals
`bake-mcp` is an independent, dedicated MCP server daemon for the **Bake Productivity Suite** (`todo`, `palmtree`, `spaghetti`, `butternut`). It connects AI coding assistants (Claude Desktop, Antigravity, Cursor, Codex) to the user's local terminal tools via the Model Context Protocol (MCP) running over standard I/O (`stdio`).

### Architectural Invariants
1. **Preserve Zero-Bloat in CLI Tools:**
   The CLI tools (`todoCli`, `palmtree`, `spaghetti`, `butternut`) must remain ultra-fast, minimal, zero-bloat UNIX utilities. They must **never** absorb async runtimes (`tokio`), networking/RPC libraries, MCP SDKs (`rmcp`), or schema generators (`schemars`).
2. **Subprocess Isolation over In-Process Linking:**
   `bake-mcp` operates primarily by executing suite binaries via non-blocking subprocesses (`tokio::process::Command`), communicating over standardized `--json` streams with isolated standard streams.
3. **Absolute Stdio Hygiene:**
   MCP protocols over `stdio` require pure JSON-RPC frames on `stdout`. Any non-protocol text on `stdout` corrupts the communication channel and crashes AI clients. All logging, tracing, and diagnostics across `bake-mcp` strictly route to `stderr`.
4. **Modular & Resilient Suite Discovery:**
   Users might install only `todo`, or only `spaghetti` and `palmtree`. `bake-mcp` dynamically discovers installed tools and gracefully adapts without failing or requiring compile-time feature matrices.

---

## 2. Architectural Debate: Tool Integration & Communication

Connecting an MCP server to a suite of independent CLI tools presents distinct architectural trade-offs:

| Criterion | Option A: In-Process Library Linking (`*-core`) | Option B: Direct State File Access (`*.json`) | Option C: Subprocess IPC via Standardized CLI Contract (Selected) |
| :--- | :--- | :--- | :--- |
| **CLI Dependency Impact** | High: Must refactor tools into `lib` + `bin`, add `tracing`/`log` facades | None: Tools unchanged | **None:** Tools only implement a lightweight `--json` output flag using existing `serde_json` |
| **Missing Tools (Partial Install)** | Poor: `bake-mcp` must statically link all tools at compile time | Moderate: Only works if state files exist on disk | **Optimal:** Dynamically detects installed binaries on `PATH` or target directories |
| **Stdio Safety (Crash Prevention)** | **Dangerous:** Any rogue `println!` in a library breaks MCP JSON-RPC | Safe: File I/O only | **Impenetrable:** Child process stdout is captured into memory; child stdout never touches parent MCP stdout |
| **Tool Scope Support** | State & logic | State only (cannot support stateless tools like `spaghetti` or `butternut`) | **Complete:** Full support for stateful (`todo`), stateless (`spaghetti`), and interactive tools (`palmtree`) |
| **Latency / Performance** | Microseconds | Sub-millisecond | **~2–5 ms** (completely negligible compared to 500–2000 ms LLM latency) |

### Selected Architecture: Subprocess IPC with Standardized CLI Contract
- **Isolation by Default:** Spawning tools via `tokio::process::Command` ensures distinct OS file descriptors. Even unhandled errors, panics, or accidental `println!` macros from CLI tools stay trapped inside `bake-mcp`'s internal pipe buffer.
- **Dynamic Availability:** At runtime, `bake-mcp` probes the system for each tool. If a tool is not installed, it is either omitted from available MCP tools or returns a clear actionable installation hint instead of crashing.
- **Independent Evolution:** CLI tools can be updated, patched, and refactored freely without rebuilding `bake-mcp`, provided the `--json` output contract remains backwards-compatible.

---

## 3. Stdio Hygiene & Logging Architecture

### The Stdio Corruption Threat
In an MCP stdio server, standard output (`stdout`) is strictly reserved for newline-delimited JSON-RPC messages:
```text
{"jsonrpc":"2.0","id":1,"result":{...}}\n
```
If any component emits arbitrary strings (e.g., `println!("Task added successfully!")` or `println!("Welcome to palmtree")`), the MCP client fails with JSON deserialization errors (`SyntaxError: Unexpected token ...`) and immediately terminates the server.

### The Resolution Strategy
1. **No `tracing` Revamp Required for CLI Tools:**
   Because `bake-mcp` captures child processes via `Stdio::piped()`, child processes do not share `bake-mcp`'s stdout. Therefore, CLI tools do **not** need to adopt `tracing` or rip out their existing logging.
2. **Strict UNIX Output Discipline for CLI Tools:**
   When `--json` is supplied to any tool:
   - `stdout` is dedicated **exclusively** to the JSON payload.
   - Any warnings, errors, or diagnostics must be written to `stderr` (`eprintln!`).
3. **`bake-mcp` Logging via `tracing` to `stderr`:**
   Adopt the battle-tested Quizzy logging pattern in `bake-mcp/src/main.rs`:
   ```rust
   tracing_subscriber::fmt()
       .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
       .with_writer(std::io::stderr)
       .with_ansi(false)
       .try_init();
   ```
4. **Transport Probe Filtering:**
   Port Quizzy's `create_stdio_transport()` duplex stream filter to intercept non-standard client probes (such as Antigravity's `server/discover`) and return clean error frames rather than dropping connections.

---

## 4. Standardized Bake Suite CLI Contract

Every CLI utility in the Bake suite exposes a standardized, machine-readable contract when invoked by automation or `bake-mcp`:

### 1. Machine Output Flag (`--json`)
- Any query command supports `--json`.
- Output is compact or formatted single-stream JSON on `stdout`.
- Exit code `0` signals success; non-zero indicates operational failure with an error message on `stderr`.

### 2. Standard Discovery Flags
- `--version` or `-v`: Emits the package version string (`todo 0.8.4`). Used by `bake-mcp` to verify installation and health.

### 3. Tool-Specific Contract Schemas

#### A. `todoCli` (`todo`)
- `todo list --json [FLAGS]`:
  ```json
  [
    {
      "id": 0,
      "text": "Implement MCP bridge",
      "completed": false,
      "tags": ["bake", "rust"],
      "files": ["bake-mcp/src/main.rs"]
    }
  ]
  ```
- `todo new <TEXT...> --json`: Emits the newly created `Task` object.
- `todo complete <ID...> --json`: Emits the updated `Task` object(s).
- `todo scan [PATHS...] --json`: Emits detected `// TODO:` comment occurrences with file, line, and text.

#### B. `spaghetti` (Topology & Diagnostics)
*Already implemented in `spaghetti/src/mcp.rs` via `--json`:*
- `spaghetti tree [PATH] --json`: Emits file count, language breakdown, SLOC, symbols, and in/out degree metrics.
- `spaghetti diag [PATH] --json`: Emits architectural diagnostics (orphans, cyclic imports, god-files, doc-coverage).

#### C. `palmtree` (Pomodoro Timer & Preferences) (DO NOT IMPLEMENT FOR NOW)
- `palmtree --config --json`:
  ```json
  {
    "version": 2,
    "work_mins": 25,
    "short_break_mins": 5,
    "long_break_mins": 15,
    "cycles": 4,
    "bell": true,
    "notification": true,
    "wait": false,
    "bell_interval_secs": 10
  }
  ```
- `palmtree presets --json`: Emits presets (`default`, `desktime`, `flow`).

#### D. `butternut` (Git Activity & Streak Grid) (DO NOT IMPLEMENT FOR NOW)
- `butternut --json`: Emits structured 52-week contribution matrices, current streak, longest streak, and total commits.

---

## 5. Dynamic Suite Discovery & Handling Missing Tools

`bake-mcp` includes a `SuiteDetector` service that checks which tools are available at runtime.

### Discovery Algorithm
1. **Search Order:**
   - Explicit environment override (e.g. `BAKE_TODO_BIN`, `BAKE_SPAGHETTI_BIN`).
   - Sibling workspace build artifacts (`../target/release/<tool>` or `../target/debug/<tool>`).
   - System `PATH` resolution via `which` or standard binary lookups.
2. **Validation Probe:**
   Runs `<binary> --version` with a 500ms timeout to confirm executable health and compatibility.

### Handling Uninstalled Tools
When an AI agent interacts with `bake-mcp`, missing tools are handled smoothly:
1. **Meta-Tool `bake_suite_info`:**
   Exposes the suite health status to the AI agent:
   ```json
   {
     "tools": {
       "todo": { "installed": true, "version": "0.8.4", "path": "/usr/local/bin/todo" },
       "spaghetti": { "installed": true, "version": "0.1.0", "path": "/usr/local/bin/spaghetti" },
       "palmtree": { "installed": false, "version": null, "path": null },
       "butternut": { "installed": false, "version": null, "path": null }
     }
   }
   ```
2. **Graceful Action Errors:**
   If the agent invokes `palm_get_config` while `palmtree` is missing, `bake-mcp` returns an informative `McpError`:
   `"Palmtree is not installed or not found on PATH. Install it via 'cargo install --path ./palmtree' to enable Pomodoro timer management."`

---

## 6. Workspace Layout

```text
rust_practice/
├── Cargo.toml                  # Workspace root
├── todoCli/                    # Synchronous, fast task CLI
│   ├── src/main.rs             # Supports --json
│   └── Cargo.toml
├── palmtree/                   # Terminal Pomodoro timer
│   ├── src/main.rs             # Supports --json
│   └── Cargo.toml
├── spaghetti/                  # Codebase topology & diagnostics
│   ├── src/
│   │   ├── main.rs             # Supports --json
│   │   └── mcp.rs              # Zero-dependency McpApi serializer
│   └── Cargo.toml
├── butternut/                  # Contribution & streak visualizer
│   ├── src/main.rs             # Supports --json
│   └── Cargo.toml
└── bake-mcp/                   # Central MCP Server Daemon
    ├── Cargo.toml
    └── src/
        ├── main.rs             # CLI entrypoint, tracing init (stderr only)
        ├── transport.rs        # Stdio duplex stream with probe filtering (from Quizzy)
        ├── server.rs           # rmcp ServerHandler & tool dispatch
        ├── detector.rs         # Dynamic binary locator & suite health prober
        ├── adapters/           # Subprocess command runners with JSON parsers
        │   ├── mod.rs
        │   ├── todo.rs
        │   ├── spaghetti.rs
        │   ├── palmtree.rs
        │   └── butternut.rs
        └── tools/              # rmcp tool parameter definitions & handlers
            ├── mod.rs
            ├── suite_tools.rs  # bake_suite_info
            ├── todo_tools.rs
            ├── spaghetti_tools.rs
            ├── palm_tools.rs
            └── butternut_tools.rs
```

---

## 7. Dependency Specification (`bake-mcp/Cargo.toml`)

`bake-mcp` isolates heavy async and schema crates from the lightweight CLI tools:

```toml
[package]
name = "bake-mcp"
version = "0.1.0"
edition = "2024"

[dependencies]
# MCP SDK & Async Runtime (tested in Quizzy)
rmcp = { version = "2.2.0", features = ["transport-io", "uuid"] }
tokio = { version = "1.48.0", features = ["rt-multi-thread", "macros", "io-util", "sync", "process"] }
schemars = "1.2.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Stderr-only diagnostic logging
tracing = "0.1.43"
tracing-subscriber = { version = "0.3.22", features = ["env-filter"] }
which = "7.0"
dirs = "6.0"
anyhow = "1.0"
```

---

## 8. Exposed MCP Tools Specification

### A. Suite Meta Tools (`tools/suite_tools.rs`)
| Tool Name | Parameters | Description | Backend Action |
| :--- | :--- | :--- | :--- |
| `bake_suite_info` | `{}` | Lists all Bake suite tools, installation status, versions, and paths. | Probes `PATH` and target directories. |

### B. Todo Tools (`tools/todo_tools.rs`)
| Tool Name | Parameters | Description | Backend Action |
| :--- | :--- | :--- | :--- |
| `todo_list` | `{ incomplete_only?: bool, tag?: String, search?: String }` | List tasks filtered by completion, tag, or keyword. | Spawns `todo list --json ...` |
| `todo_add` | `{ text: String, tags?: Vec<String> }` | Create a new task with optional tags. | Spawns `todo new "<text>" --json` |
| `todo_complete` | `{ task_ids: Vec<usize> }` | Mark tasks completed by ID. | Spawns `todo complete <id...> --json` |
| `todo_scan` | `{ path?: String }` | Scan files for `// TODO:` comments to convert into tasks. | Spawns `todo scan [path] --json` |

### C. Spaghetti Tools (`tools/spaghetti_tools.rs`)
| Tool Name | Parameters | Description | Backend Action |
| :--- | :--- | :--- | :--- |
| `spaghetti_tree` | `{ path?: String, dep?: bool, symbols?: bool }` | Analyze project file tree, language metrics, SLOC, and dependency degree. | Spawns `spaghetti tree [path] --json` |
| `spaghetti_diagnostics`| `{ path?: String, wall?: bool }` | Run architectural linter (detect orphan files, circular imports, god-files). | Spawns `spaghetti diag [path] --json` |

### D. Palmtree Tools (`tools/palm_tools.rs`)
| Tool Name | Parameters | Description | Backend Action |
| :--- | :--- | :--- | :--- |
| `palm_get_config` | `{}` | Retrieve current Pomodoro work/break durations and audio settings. | Spawns `palmtree --config --json` |
| `palm_get_presets` | `{}` | Retrieve available productivity presets (Standard, DeskTime, Flow). | Spawns `palmtree presets --json` |

### E. Butternut Tools (`tools/butternut_tools.rs`)
| Tool Name | Parameters | Description | Backend Action |
| :--- | :--- | :--- | :--- |
| `butternut_stats` | `{ path?: String }` | Retrieve Git contribution frequency, streaks, and activity matrix. | Spawns `butternut --json` |

---

## 9. Implementation Roadmap

### Phase 1: CLI Contract Alignment (Minimal CLI updates)
1. Verify `spaghetti` already emits `--json` (complete in `spaghetti/src/mcp.rs`).
2. Add `--json` flag support to `todoCli` for `list`, `new`, and `complete`.
3. Add `--json` flag to `palmtree` for `--config`.
4. Ensure all tools write diagnostics exclusively to `stderr` when `--json` is active.

### Phase 2: `bake-mcp` Transport & Foundation
1. Configure `bake-mcp/Cargo.toml` with dependencies.
2. Implement `transport.rs` porting Quizzy's `create_stdio_transport()` with `server/discover` probe interceptor.
3. Setup `main.rs` with `tracing_subscriber` directed strictly to `std::io::stderr`.
4. Implement `detector.rs` for dynamic binary resolution.

### Phase 3: Adapters & Tool Routers
1. Implement async subprocess adapters (`adapters/todo.rs`, `adapters/spaghetti.rs`, etc.) with timeout handling and error mapping.
2. Register tools with `rmcp` using `nullable_to_anyof` schema transformers.
3. Wire graceful fallbacks for missing CLI binaries.

### Phase 4: Integration Verification
1. Configure in client agent settings (`~/.gemini/antigravity/mcp.json` or Claude Desktop config):
   ```json
   {
     "mcpServers": {
       "bake": {
         "command": "cargo",
         "args": ["run", "--release", "--manifest-path", "rust_practice/bake-mcp/Cargo.toml"]
       }
     }
   }
   ```
2. Verify with test interactions:
   - *"Check my bake suite status."*
   - *"Scan the rust_practice repo with spaghetti and list open todos for the largest god-file."*
