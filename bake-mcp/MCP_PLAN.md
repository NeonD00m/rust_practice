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

## 4. Standardized Bake Suite CLI Contract & New Capabilities

Every CLI utility in the Bake suite exposes a standardized, machine-readable contract when invoked with `--json`:

### 1. General Invariants
- Any query or mutation command supports `--json`.
- Output is a single valid JSON document on `stdout`.
- Diagnostics, progress, and warnings route strictly to `stderr` (`eprintln!`).
- Exit code `0` signals success; non-zero signals failure with human/JSON details on `stderr`.

---

### 2. Tool Contracts (Updated with Latest Features from `my_todo.json`)

#### A. `todoCli` (`todo`) — Version 0.9.0 Features
*Recent enhancements incorporated:*
- **Targeting & Personal Lists:** `--personal` for system-wide personal todo list (`~/.config/todo/my_todo.json`), multiple `-p, --path <PATH>` overrides.
- **Discovery:** Standalone `todo discover` and `-d, --discover` on `list` with `--depth <N>` to recursively locate and aggregate nested `my_todo.json` files.
- **Fast Creation:** `todo new <TEXT...> -t <TAG...> -a <FILE...> -v` with automatic tag deduplication and ID return.
- **Query Mutations:** `-q, --query` support across `complete`, `undo`, `delete`, `add` (tags), `attach` (files), and `detach`.
- **Query Filter Flags:** `-t <TAG>`, `--not <TAG>` (negative tag filtering), `-s <TERM>`, `-i` (incomplete only), `-c` (complete only).
- **Inward Scanners:** `todo scan [PATHS...]` (converts `// TODO:` comments to tasks) and `todo import [PATHS...]` (imports markdown checklists `- [ ]`).

##### CLI Subcommands & JSON Output Schemas:
* `todo list --json [FLAGS]`:
  ```json
  [
    {
      "id": 0,
      "text": "Refactor parser",
      "completed": false,
      "tags": ["v0.9.0", "core"],
      "files": ["src/main.rs"]
    }
  ]
  ```
* `todo discover --json [--depth <N>]`:
  ```json
  [
    {
      "path": "C:/Projects/rust_practice/todoCli/my_todo.json",
      "total_tasks": 9,
      "incomplete_tasks": 0,
      "completed_tasks": 9
    },
    {
      "path": "C:/Projects/rust_practice/spaghetti/my_todo.json",
      "total_tasks": 4,
      "incomplete_tasks": 0,
      "completed_tasks": 4
    }
  ]
  ```
* `todo new <TEXT...> --json [-t <TAG...>] [-a <FILE...>] [--personal]`:
  ```json
  {
    "id": 10,
    "text": "Add MCP bridge adapter",
    "completed": false,
    "tags": ["bake", "mcp"],
    "files": ["bake-mcp/src/main.rs"]
  }
  ```
* `todo complete [IDs...] --json [-q] [-t <TAG>] [--not <TAG>] [-s <TERM>]`:
  ```json
  {
    "completed_count": 2,
    "tasks": [
      { "id": 4, "text": "...", "completed": true, "tags": [...], "files": [...] }
    ]
  }
  ```
* `todo scan [PATHS...] --json`:
  ```json
  [
    {
      "file": "src/parser.rs",
      "line": 42,
      "comment": "Support raw strings in scanner"
    }
  ]
  ```

---

#### B. `spaghetti` — Version 0.2.0 Features
*Recent enhancements incorporated:*
- **Expanded Languages (18+):** `c`, `cpp`, `py`, `js`, `ts`, `lua`, `luau`, `rs`, `go`, `java`, `cs`, `rb`, `swift`, `kt`, `php`, `hs`, `html`, `css`.
- **Custom Extension Ingestion:** `--include <EXT...>` to parse and track non-standard or domain-specific file types (e.g. `--include proto,zig,wgsl`).
- **Compact Basenames:** `--short` to compress filenames in dependency graphs (avoiding `mod.rs` nesting clutter).
- **LOC Analysis:** `spaghetti loc` with `--flat` (global table) and `--limit <N>`.
- **Architectural Diagnostics:** `-Wall`, `-Worphan`, `-Wcycle`, `-Wgod`, `-Wdoc`, `--god-loc <N>`, `--doc-ratio <N>`.
- **Symbol Extraction:** `--outline` / `--symbols` (extracts structs, traits, classes, interfaces).

##### CLI Subcommands & JSON Output Schemas:
* `spaghetti tree [PATH] --json [--dep] [--short] [--loc] [--symbols] [-l <LANGS>] [--include <EXTS>]`:
  ```json
  {
    "total_files": 42,
    "ignored_count": 15,
    "files": [
      {
        "path": "C:/Projects/rust_practice/spaghetti/src/main.rs",
        "rel_path": "src/main.rs",
        "language": "Rust",
        "sloc": 355,
        "symbols": [
          { "name": "print_help", "kind": "fn", "line": 16 },
          { "name": "main", "kind": "fn", "line": 60 }
        ],
        "in_degree": 0,
        "out_degree": 5
      }
    ]
  }
  ```
* `spaghetti loc [PATH] --json [--flat] [--limit <N>]`:
  ```json
  {
    "total_sloc": 8420,
    "files": [
      { "rel_path": "src/main.rs", "language": "Rust", "sloc": 355 },
      { "rel_path": "src/scanner/loc.rs", "language": "Rust", "sloc": 290 }
    ]
  }
  ```
* `spaghetti diag [PATH] --json [-Wall] [--god-loc <N>] [--doc-ratio <N>]`:
  ```json
  {
    "total_warnings": 2,
    "warnings": [
      {
        "tag": "orphan",
        "message": "File is not imported by any other module",
        "file": "src/unused_helper.rs",
        "level": "warning"
      },
      {
        "tag": "god-file",
        "message": "SLOC (1240) exceeds god-file threshold (1000)",
        "file": "src/core/engine.rs",
        "level": "warning"
      }
    ]
  }
  ```

---

#### C. `butternut` — Version 0.2.0 Features
*Recent enhancements incorporated:*
- **Multi-Source Support:** Local Git Repository (`-p, --path <PATH>`) vs. GitHub GraphQL API (`-u, --user <USERNAME>`).
- **Author Filtering:** `--all` (all authors), `--me` (uses `git config user.name`), `--author <AUTHOR>` / `-u <USER>`.
- **Streak & Today's Activity Metrics:** Computes `current_streak`, `longest_streak`, `today_count`, and `is_active_today`.
- **Flexible Timeframes:** `-y, --year <YYYY>`, `--since <YYYY-MM-DD>`, `--until <YYYY-MM-DD>`, `-w, --weeks <N>`.

##### CLI Subcommands & JSON Output Schemas:
* `butternut --json [PATH | USERNAME] [OPTIONS]`:
  ```json
  {
    "source": "repo",
    "title_label": "Repository",
    "title_value": "rust_practice (author: max)",
    "total_count": 184,
    "unit": "commits",
    "timeframe_desc": "in the last 52 weeks",
    "streaks": {
      "current_streak": 6,
      "longest_streak": 19,
      "today_count": 4,
      "is_active_today": true
    },
    "weeks": [
      {
        "days": [
          { "date": "2026-10-18", "count": 3, "level": 2 },
          { "date": "2026-10-19", "count": 0, "level": 0 }
        ]
      }
    ]
  }
  ```

---

#### D. `palmtree` (Pomodoro Timer & Productivity Sessions)
*Features incorporated:*
- **Session Durations & Ratios:** Configurable `work_mins`, `short_break_mins`, `long_break_mins`, `cycles`.
- **Presets:** Standard Pomodoro (25/5/15/4), `DeskTime` (52/17/17/9), `Flow` (90/20/20/4).
- **Controls & Audio:** `bell`, `notification`, `wait` mode, `bell_interval_secs`.
- **Session Offset:** `-a, --ago <MINUTES>` to credit already elapsed time.

##### CLI Subcommands & JSON Output Schemas:
* `palmtree --config --json`:
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
* `palmtree presets --json`:
  ```json
  {
    "default": { "work_mins": 25, "short_break_mins": 5, "long_break_mins": 15, "cycles": 4 },
    "desktime": { "work_mins": 52, "short_break_mins": 17, "long_break_mins": 17, "cycles": 9 },
    "flow": { "work_mins": 90, "short_break_mins": 20, "long_break_mins": 20, "cycles": 4 }
  }
  ```

---

## 5. Dynamic Suite Discovery & Handling Missing Tools

`bake-mcp` includes a `SuiteDetector` service that checks which tools are available at runtime.

### Discovery Algorithm
1. **Search Order:**
   - Explicit environment override (e.g. `BAKE_TODO_BIN`, `BAKE_SPAGHETTI_BIN`, `BAKE_BUTTERNUT_BIN`, `BAKE_PALMTREE_BIN`).
   - Sibling workspace build artifacts (`../target/release/<tool>` or `../target/debug/<tool>`).
   - System `PATH` resolution via `which` or standard binary lookups.
2. **Validation Probe:**
   Runs `<binary> --version` with a 500ms timeout to confirm executable health and compatibility.

### Handling Uninstalled Tools
1. **Meta-Tool `bake_suite_info`:**
   Exposes the suite health status to the AI agent:
   ```json
   {
     "tools": {
       "todo": { "installed": true, "version": "0.9.0", "path": "C:/Users/user/.cargo/bin/todo.exe" },
       "spaghetti": { "installed": true, "version": "0.2.0", "path": "C:/Users/user/.cargo/bin/spaghetti.exe" },
       "butternut": { "installed": true, "version": "0.2.0", "path": "C:/Users/user/.cargo/bin/butternut.exe" },
       "palmtree": { "installed": false, "version": null, "path": null }
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
├── todoCli/                    # Synchronous, fast task CLI (v0.9.0)
│   ├── src/main.rs             # Supports --json
│   └── Cargo.toml
├── palmtree/                   # Terminal Pomodoro timer
│   ├── src/main.rs             # Supports --json
│   └── Cargo.toml
├── spaghetti/                  # Codebase topology & diagnostics (v0.2.0)
│   ├── src/
│   │   ├── main.rs             # Supports --json
│   │   └── mcp.rs              # Zero-dependency McpApi serializer
│   └── Cargo.toml
├── butternut/                  # Contribution & streak visualizer (v0.2.0)
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
            ├── todo_tools.rs   # todo_list, todo_add, todo_complete, todo_scan, etc.
            ├── spaghetti_tools.rs # spaghetti_tree, spaghetti_loc, spaghetti_diagnostics
            ├── palm_tools.rs   # palm_get_config, palm_get_presets, palm_prepare_task
            └── butternut_tools.rs # butternut_repo_activity, butternut_github_activity, butternut_streak_status
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

## 8. Exposed MCP Tools Specification (Detailed API Schemas)

### A. Suite Meta Tools (`tools/suite_tools.rs`)

#### `bake_suite_info`
* **Description:** Inspects local system environment, discovers installed Bake suite tools, and reports binary paths, versions, and capability descriptors.
* **Parameters:** `{}`
* **Return Schema:**
  ```json
  {
    "installed_count": 3,
    "tools": {
      "todo": { "installed": true, "version": "0.9.0", "path": "..." },
      "spaghetti": { "installed": true, "version": "0.2.0", "path": "..." },
      "butternut": { "installed": true, "version": "0.2.0", "path": "..." },
      "palmtree": { "installed": false, "version": null, "path": null }
    }
  }
  ```

---

### B. Todo Tools (`tools/todo_tools.rs`)

#### 1. `todo_list`
* **Description:** Lists tasks from the active project list, personal device list, or aggregates across discovered project files, with rich query and negative filtering.
* **Parameters (`TodoListRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  #[schemars(transform = nullable_to_anyof)]
  pub struct TodoListRequest {
      pub path: Option<String>,          // Path to specific my_todo.json file
      pub personal: Option<bool>,        // Use system-wide personal todo list
      pub discover: Option<bool>,        // Recursively find and aggregate all todo lists
      pub depth: Option<u32>,            // Max recursion depth for discovery (default: 3)
      pub incomplete_only: Option<bool>, // Filter only incomplete tasks
      pub complete_only: Option<bool>,   // Filter only completed tasks
      pub tag: Option<String>,           // Filter tasks that have this tag
      pub not_tag: Option<String>,       // Filter tasks that DO NOT have this tag
      pub search: Option<String>,        // Filter tasks containing search keyword
      pub page: Option<u32>,             // Page number (10 items per page)
  }
  ```

#### 2. `todo_add`
* **Description:** Creates a new task with optional tags, attached files, and target list scope. Guarantees tag deduplication.
* **Parameters (`TodoAddRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  #[schemars(transform = nullable_to_anyof)]
  pub struct TodoAddRequest {
      pub text: String,                  // Task description
      pub tags: Option<Vec<String>>,     // Initial tags to assign
      pub files: Option<Vec<String>>,    // Associated file paths
      pub personal: Option<bool>,        // Save to personal list
      pub path: Option<String>,          // Target specific todo file
  }
  ```

#### 3. `todo_complete` / `todo_undo`
* **Description:** Marks one or more tasks as completed or incomplete. Supports explicit task IDs or bulk query-based selection (`-q`).
* **Parameters (`TodoMutationRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  #[schemars(transform = nullable_to_anyof)]
  pub struct TodoMutationRequest {
      pub task_ids: Option<Vec<usize>>,  // Explicit task ID numbers
      pub query_tag: Option<String>,     // Bulk select by tag
      pub query_not_tag: Option<String>, // Bulk select excluding tag
      pub query_search: Option<String>,  // Bulk select by keyword
      pub personal: Option<bool>,
      pub path: Option<String>,
  }
  ```

#### 4. `todo_delete`
* **Description:** Deletes tasks by ID or by query matching.
* **Parameters:** Same as `TodoMutationRequest`.

#### 5. `todo_tag_manage`
* **Description:** Adds or removes tags from existing tasks.
* **Parameters (`TodoTagManageRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  #[schemars(transform = nullable_to_anyof)]
  pub struct TodoTagManageRequest {
      pub action: String,                // "add" or "remove"
      pub tags: Vec<String>,             // Tags to modify
      pub task_id: Option<usize>,        // Target task ID
      pub query_tag: Option<String>,     // Target tasks by tag query
      pub personal: Option<bool>,
      pub path: Option<String>,
  }
  ```

#### 6. `todo_file_attach`
* **Description:** Attaches or detaches file paths to/from a task.
* **Parameters (`TodoFileAttachRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  #[schemars(transform = nullable_to_anyof)]
  pub struct TodoFileAttachRequest {
      pub action: String,                // "attach" or "detach"
      pub files: Vec<String>,            // File paths
      pub task_id: usize,                // Target task ID
      pub personal: Option<bool>,
      pub path: Option<String>,
  }
  ```

#### 7. `todo_discover`
* **Description:** Recursively discovers all `my_todo.json` files in the workspace tree with task metrics.
* **Parameters (`TodoDiscoverRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  #[schemars(transform = nullable_to_anyof)]
  pub struct TodoDiscoverRequest {
      pub root_path: Option<String>,     // Root directory to scan
      pub depth: Option<u32>,            // Max recursion depth (default: 4)
  }
  ```

#### 8. `todo_scan`
* **Description:** Scans codebase source files for `// TODO:` comments to convert or review.
* **Parameters (`TodoScanRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  #[schemars(transform = nullable_to_anyof)]
  pub struct TodoScanRequest {
      pub paths: Option<Vec<String>>,    // Files or directories to scan
  }
  ```

#### 9. `todo_import`
* **Description:** Parses Markdown checklists (`- [ ]`, `- [x]`) from documentation files and imports them into tasks.
* **Parameters (`TodoImportRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  #[schemars(transform = nullable_to_anyof)]
  pub struct TodoImportRequest {
      pub markdown_paths: Vec<String>,   // Markdown files to import
      pub personal: Option<bool>,
      pub path: Option<String>,
  }
  ```

---

### C. Spaghetti Tools (`tools/spaghetti_tools.rs`)

#### 1. `spaghetti_tree`
* **Description:** Generates an architectural code tree or import dependency graph with SLOC, symbols, compact basenames, and language/extension filters.
* **Parameters (`SpaghettiTreeRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  #[schemars(transform = nullable_to_anyof)]
  pub struct SpaghettiTreeRequest {
      pub path: Option<String>,              // Target directory (default: current dir)
      pub dep: Option<bool>,                 // Logical import dependency graph mode
      pub short: Option<bool>,               // Compact basenames (reduces mod.rs clutter)
      pub loc: Option<bool>,                 // Include physical SLOC per file
      pub symbols: Option<bool>,             // Extract symbols (structs, traits, classes)
      pub languages: Option<Vec<String>>,    // Filter by language (c, cpp, py, js, ts, lua, rs, go, java, php, hs, html, css)
      pub extensions: Option<Vec<String>>,   // Filter by extension (e.g. ts, tsx)
      pub include_extensions: Option<Vec<String>>, // Custom extensions to parse (e.g. proto, zig, wgsl)
      pub max_depth: Option<u32>,            // Max directory traversal depth
      pub no_ignore: Option<bool>,           // Ignore .gitignore rules
  }
  ```

#### 2. `spaghetti_loc`
* **Description:** Ranks codebase files by physical SLOC, supporting language grouping or global flat tables with limit thresholds.
* **Parameters (`SpaghettiLocRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  #[schemars(transform = nullable_to_anyof)]
  pub struct SpaghettiLocRequest {
      pub path: Option<String>,
      pub flat: Option<bool>,                // Flatten ranking into global table
      pub limit: Option<usize>,              // Return top N largest files
      pub languages: Option<Vec<String>>,
      pub extensions: Option<Vec<String>>,
      pub include_extensions: Option<Vec<String>>,
  }
  ```

#### 3. `spaghetti_diagnostics`
* **Description:** Runs the compiler-style architectural linter detecting orphan files, cyclic imports, god-files, and under-documented modules.
* **Parameters (`SpaghettiDiagRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  #[schemars(transform = nullable_to_anyof)]
  pub struct SpaghettiDiagRequest {
      pub path: Option<String>,
      pub wall: Option<bool>,                // Enable all diagnostic linters
      pub warn_orphan: Option<bool>,         // Flag unreferenced modules
      pub warn_cycle: Option<bool>,          // Flag circular import loops
      pub warn_god: Option<bool>,            // Flag bloated god-files
      pub god_loc_threshold: Option<usize>,  // SLOC threshold for god files (default: 1000)
      pub warn_doc: Option<bool>,            // Flag low doc-comment ratios
      pub doc_ratio_threshold: Option<f32>,  // Minimum doc comment ratio in % (default: 5.0)
  }
  ```

---

### D. Butternut Tools (`tools/butternut_tools.rs`)

#### 1. `butternut_repo_activity`
* **Description:** Analyzes a local Git repository to extract commit frequency, daily contribution grids, and author streaks.
* **Parameters (`ButternutRepoRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  #[schemars(transform = nullable_to_anyof)]
  pub struct ButternutRepoRequest {
      pub path: Option<String>,              // Path to Git repo (defaults to current dir)
      pub author: Option<String>,            // Specific commit author name or email
      pub me: Option<bool>,                  // Filter to local git user (user.name)
      pub all_authors: Option<bool>,         // Include all contributors (default: true)
      pub year: Option<i32>,                 // Filter to specific year (e.g. 2026)
      pub since: Option<String>,             // Start date (YYYY-MM-DD)
      pub until: Option<String>,             // End date (YYYY-MM-DD)
      pub weeks: Option<usize>,              // Number of weeks (default: 52)
  }
  ```

#### 2. `butternut_github_activity`
* **Description:** Fetches GitHub contribution calendars and streaks via the `gh` CLI GraphQL API.
* **Parameters (`ButternutGitHubRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  #[schemars(transform = nullable_to_anyof)]
  pub struct ButternutGitHubRequest {
      pub username: Option<String>,          // GitHub username (default: authenticated gh user)
      pub year: Option<i32>,
      pub since: Option<String>,
      pub until: Option<String>,
      pub weeks: Option<usize>,
  }
  ```

#### 3. `butternut_streak_status`
* **Description:** Fast status check for productivity agents to verify if the user's coding streak is active today, the current streak length, and today's commit count.
* **Parameters (`ButternutStreakRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  #[schemars(transform = nullable_to_anyof)]
  pub struct ButternutStreakRequest {
      pub path: Option<String>,              // Local git repo path (optional)
      pub username: Option<String>,          // GitHub username (optional)
  }
  ```
* **Return Schema:**
  ```json
  {
    "is_active_today": true,
    "current_streak": 7,
    "longest_streak": 21,
    "today_contributions": 3,
    "status_message": "Streak active! 7 days in a row (3 contributions today)."
  }
  ```

---

### E. Palmtree Tools (`tools/palm_tools.rs`)

#### 1. `palm_get_config`
* **Description:** Retrieves active Pomodoro work/break durations, cycle counts, sound alerts, and wait preferences.
* **Parameters:** `{}`

#### 2. `palm_get_presets`
* **Description:** Retrieves available productivity ratios (Standard Pomodoro, DeskTime 52/17, Flow 90/20).
* **Parameters:** `{}`

#### 3. `palm_prepare_task`
* **Description:** Resolves a task from `todo` formatted specifically for pipe-driving palmtree focus sessions (`todo print <id> | palmtree start`).
* **Parameters (`PalmPrepareTaskRequest`):**
  ```rust
  #[derive(Deserialize, JsonSchema)]
  pub struct PalmPrepareTaskRequest {
      pub task_id: usize,
      pub path: Option<String>,
      pub personal: Option<bool>,
  }
  ```

---

## 9. Implementation Roadmap

### Phase 1: CLI Contract Alignment (Minimal CLI Updates)
1. **`spaghetti`**: Verify `--json` outputs match the schema (complete in `spaghetti/src/mcp.rs`).
2. **`todoCli`**:
   - Add `--json` flag to `list`, `new`, `complete`, `discover`, `scan`.
   - Ensure all diagnostic messages route to `stderr` (`eprintln!`).
3. **`butternut`**:
   - Add `--json` flag outputting `ContributionGrid` + `StreakStats`.
4. **`palmtree`**:
   - Add `--json` flag to `--config` and `presets`.

### Phase 2: `bake-mcp` Transport & Foundation
1. Configure `bake-mcp/Cargo.toml` with dependencies (`rmcp 2.2.0`, `tokio 1.48`, `schemars`, `tracing`).
2. Implement `transport.rs` porting Quizzy's `create_stdio_transport()` with `server/discover` probe interceptor.
3. Setup `main.rs` with `tracing_subscriber` directed strictly to `std::io::stderr`.
4. Implement `detector.rs` for dynamic binary resolution (`todo`, `spaghetti`, `butternut`, `palmtree`).

### Phase 3: Adapters & Tool Routers
1. Implement async subprocess adapters in `bake-mcp/src/adapters/` with timeout handling and error mapping.
2. Implement `rmcp` tool handlers in `bake-mcp/src/tools/` using `nullable_to_anyof` schema transformers.
3. Wire graceful fallbacks for missing CLI binaries.

### Phase 4: Integration Verification
1. Register `bake-mcp` in local agent configs (`~/.gemini/antigravity/mcp.json` or Claude Desktop):
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
2. Verify end-to-end tool flows:
   - *"Check my daily streak with butternut and report my current progress."*
   - *"Find the largest god-file using spaghetti, then create a todo item in my personal list to refactor it."*
