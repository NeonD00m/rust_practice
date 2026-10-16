# Spaghetti (`spaghetti`) — Architecture & Implementation Plan

> **"Untangle the Pasta: Zero-Bloat Codebase Topology, Dependency Mapping & Onboarding Diagnostics."**

---

## 1. Mission Statement

Modern codebases devolve quickly into tangled webs of inter-file dependencies—"spaghetti code" where nobody truly knows the entrypoints, the coupling hot-spots, or what breaks when a core module changes. Existing tools for understanding codebase architecture are either heavy language servers (requiring massive memory, complex toolchains, and sluggish startup) or simplistic directory tree utilities (which display directory layout without filtering out non-code noise or showing logical code dependencies).

**`spaghetti`** is a fast, zero-bloat command-line code cartographer and architectural analyzer written in Rust. It joins the `rust_practice` suite (`todo`, `palmtree`) under the strict UNIX philosophy:
- **Instantaneous Execution:** Starts in milliseconds, scans thousands of files without background daemon overhead or language server setups.
- **Smart Code-Only Cartography:** Automatically respects `.gitignore` and filters out non-code noise (`.json`, `.csv`, `.lock`, `.md`, etc.) to present pure code topology.
- **Dual-Tree Exploration:** Seamlessly toggles between clean **Filesystem Code Trees** and logical **Dependency Graph Trees** (`--dep`).
- **Same-Line Column Alignment:** Inlines physical SLOC and extracted entity definitions (structs, classes, traits) on the same line in dedicated columns without vertical line clutter.
- **Cycle-Resilient Algorithmic Core:** Identifies natural architectural roots, decomposes Strongly Connected Components (SCCs), and prevents circular dependency recursion using cycle back-edge annotations.
- **Compiler-Style Warning Diagnostics (`-Wall`):** Configurable `-W` flags for dead code (`-Worphan`), circular imports (`-Wcycle`), god files (`-Wgod`, default 1000 LOC), and under-documented files (`-Wdoc`).
- **High-Contrast Accessibility:** Includes built-in colorblind-safe distinct color palettes so files and symbols never blend together visually.
- **Pipe-First Ergonomics:** Seamlessly pipes into `todo`, `palmtree`, and downstream MCP/AI agents via compact raw text and structured JSON streams.

---

## 2. Workspace & Crate Structure

`spaghetti` resides as an independent crate in the `rust_practice` monorepo:

```text
rust_practice/
├── Cargo.toml                  # Workspace virtual manifest
├── todoCli/                    # Synchronous task manager
├── pomodoro/                   # Terminal Pomodoro timer (palmtree)
├── bake-mcp/               # Model Context Protocol bridge daemon
└── spaghetti/                  # NEW: Codebase Cartographer & Dependency Mapper
    ├── Cargo.toml
    └── src/
        ├── main.rs             # CLI parsing (quoteless, short shortcuts), terminal dispatch
        ├── core/
        │   ├── mod.rs
        │   ├── config.rs       # Path filters, depth limits, ignored extensions, .gitignore parser
        │   ├── types.rs        # FileNode, ImportEdge, Symbol, CodeMetrics
        │   └── palette.rs      # High-contrast colorblind-safe deterministic color hashing
        ├── scanner/
        │   ├── mod.rs          # Recursive file traversal & language detection
        │   ├── gitignore.rs    # Fast zero-dependency .gitignore rule evaluator
        │   ├── loc.rs          # Physical LOC, SLOC, comment & blank line counter
        │   ├── symbols.rs      # Fast regex/state-machine symbol extractor (structs/classes/traits)
        │   └── imports/        # Per-language import statement parsers
        │       ├── mod.rs
        │       ├── c_cpp.rs    # #include "..." vs <...>
        │       ├── python.rs   # import x, from x import y, relative imports
        │       ├── js_ts.rs    # import, export, require, dynamic imports (.js/.mjs/.ts/.tsx)
        │       ├── lua.rs      # require("..."), Luau module resolution
        │       └── rust.rs     # mod foo;, use crate::..., super::..., self::...
        ├── graph/
        │   ├── mod.rs
        │   ├── resolver.rs     # File-to-file path resolution & alias expansion
        │   ├── dependency.rs   # Directed graph construction (Adjacency list)
        │   ├── cycles.rs       # Tarjan's SCC & 3-Color DFS cycle breaking
        │   └── roots.rs        # Topological source detection & entrypoint heuristics
        ├── render/
        │   ├── mod.rs
        │   ├── columns.rs      # Aligned column layout engine (Tree branch | LOC | Symbols)
        │   ├── tree.rs         # Terminal box-drawing (├──, └──, │   ) with cycle badges
        │   ├── loc_rank.rs     # Language-grouped and flat LOC ranking views
        │   ├── diag.rs         # Compiler-style warning reporting (-W flags)
        │   └── raw.rs          # Pipeable raw output and JSON streaming
        └── mcp.rs              # Headless query API for bake-mcp integration
```

### Dependency Specification (`spaghetti/Cargo.toml`)
Following the zero-bloat invariant from `AGENTS.md`:
```toml
[package]
name = "spaghetti"
version = "0.1.0"
edition = "2024"

[dependencies]
# Zero heavy TUI engines: light, fast terminal primitives
crossterm = "0.28"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

---

## 3. Multi-Language Scanner & Filter Architecture

Rather than relying on heavy AST parsers or multi-megabyte language server binaries, `spaghetti` employs streaming line readers (`std::io::BufReader`) paired with dedicated zero-allocation pattern matchers.

### A. Non-Code Filtering & Gitignore Integration
- **Code-Only Filtering by Default:** Automatically suppresses non-source files (`.json`, `.csv`, `.lock`, `.md`, `.txt`, `.yaml`, `.toml`, `.xml`, image assets, binaries).
- **`.gitignore` Resolution:** By default, parses `.gitignore` in the target project root and skips ignored build folders (`target/`, `node_modules/`, `dist/`, `.venv/`, `build/`).
- **Flag `--no-ignore`:** Bypasses `.gitignore` filtering if the user wants an exhaustive scan.

### B. Supported Languages & Import Syntax

| Language | Extensions Recognized | Import Patterns Handled | Relative Resolution Logic |
| :--- | :--- | :--- | :--- |
| **C / C++** | `.c`, `.h`, `.cc`, `.cpp`, `.cxx`, `.hpp`, `.hxx`, `.inl` | `#include "path/file.h"`<br>`#include <path/file.h>` | Local headers (`"..."`) resolve relative to current file then `-I` search paths. System headers (`<...>`) tagged as external. |
| **Python** | `.py`, `.pyi` | `import module`<br>`from module import item`<br>`from .relative import item`<br>`from ..parent import item` | Dots counted for directory backtracking; resolves against project root, `sys.path` folders, or `__init__.py` packages. |
| **JS / TS** | `.js`, `.mjs`, `.cjs`, `.jsx`, `.ts`, `.mts`, `.cts`, `.tsx` | `import x from './path'`<br>`import('./path')`<br>`const x = require('./path')`<br>`export * from './path'` | Handles omitted extensions (`./util` -> `./util.ts`, `./util.tsx`, `./util.js`, `./util/index.ts`). Skips `node_modules`. |
| **Lua / Luau**| `.lua`, `.luau` | `require("path.module")`<br>`require(script.Parent.Module)` | Dot/slash conversion (`require("core.utils")` -> `core/utils.lua` or `core/utils/init.lua`). |
| **Rust** | `.rs` | `mod child_mod;`<br>`use crate::path;`<br>`use super::path;`<br>`use self::path;` | Resolves module file trees: `foo.rs` or `foo/mod.rs`. Resolves `crate::` from `src/main.rs` or `src/lib.rs`. |

---

## 4. Algorithmic Planning: Graph Topology & Cycle-Breaking

Software dependency graphs are **Directed Graphs with Cycles**, whereas console trees are **Directed Acyclic Trees**.

### A. Root Node Selection (Finding Natural Entrypoints)
1. **Degree Metrics:**
   - In-Degree $d_{in}(v)$: Number of files that import $v$ (incoming dependencies).
   - Out-Degree $d_{out}(v)$: Number of files that $v$ imports (outgoing dependencies).
2. **Source Detection:**
   - Natural roots are files where $d_{in}(v) = 0$ (no internal file imports them). These represent CLI entry points, binary mains, API entry routes, or root scripts.
3. **Tie-Breaking & Heuristics:**
   If multiple candidate roots exist, rank them by:
   - Primary: Standard naming conventions (`main.*`, `index.*`, `lib.*`, `app.*`, `mod.*`).
   - Secondary: High out-degree $d_{out}$ (files that orchestrate the largest downstream trees).
4. **Disconnected Clusters / Orphan Roots:**
   Subgraphs with disjoint components are surfaced sequentially as distinct root trees.

### B. Cycle Detection & Tarjan's SCC Reduction
Circular dependencies ($A \to B \to A$) cause infinite loops in naive tree renderers. `spaghetti` solves this via a two-layer cycle guard:

1. **3-Color DFS Graph Traversal:**
   - **White (Unvisited):** Node has not been touched.
   - **Gray (Active in Current Path):** Node is on the active traversal call stack.
   - **Black (Finished):** Node and all descendants have been completely processed.
2. **Cycle Identification:**
   During DFS, if an edge $(u, v)$ targets a **Gray** node $v$, a back-edge (cycle) is detected.
3. **Cycle-Breaking Tree Annotation:**
   - The recursive traversal immediately terminates on that branch.
   - Instead of duplicating the subtree or looping, the node is rendered with a cyclical indicator:
     `└── auth.ts (cycle ⟲ user.ts) [WARNING: -Wcycle]`
4. **Strongly Connected Component (SCC) Analysis:**
   Using Tarjan's linear-time $O(V + E)$ algorithm, `spaghetti` identifies circular clusters and prints the exact cycle ring in the `diag` command.

---

## 5. Visual Styling & Colorblind-Safe Palette

To prevent visual blending for colorblind users and maintain readable contrast across terminal color schemes:
- **Palette Architecture:** Adopts the scientific **Okabe-Ito / Wong** colorblind-safe palette using high-contrast ANSI 256 / TrueColor codes:
  - **Cyan** (`#56B4E9` / ANSI 75)
  - **Amber / Orange** (`#E69F00` / ANSI 214)
  - **Royal Blue** (`#0072B2` / ANSI 33)
  - **Magenta / Purple** (`#CC79A7` / ANSI 175)
  - **Clean White / Gray** (for structure branches and lines)
  - Avoids conflicting red/green pairs.
- **Deterministic Color Hashing (`--color-distinct`):**
  Hashes file extensions, languages, or symbol names into this palette so adjacent files or classes are instantly distinguishable without straining.
- Toggleable via `--color-distinct` or global config `~/.config/spaghetti/config.json`.

---

## 6. CLI Syntax & Command Specification

`spaghetti` maintains a clean, lean set of core subcommands and uses composable modifier flags:

### Primary Subcommands

| Shortcut | Subcommand | Arguments | Description |
| :---: | :--- | :--- | :--- |
| `t` | `tree` | `[PATH/QUERY...] [FLAGS]` | Renders project tree. Default: Code Filesystem Tree. With `--dep`: Logical Dependency Graph. |
| `l` | `loc` | `[PATH/QUERY...] [FLAGS]` | Language-grouped LOC ranking sorted most-to-least. With `--flat`: Global ranking. With `--outline`: Nested symbols. |
| `d` | `diag` | `[PATH] [FLAGS]` | Unified compiler-style architectural diagnostics (`-Wall`, `-Worphan`, `-Wcycle`, `-Wgod`, etc.). |
| `p` | `print` | `[QUERY...] [FLAGS]` | Fast, unadorned code file listing (analogous to `todo print`). |
| `-h` | `help` | `[COMMAND]` | Displays command help and usage. |

### Global Modifier Flags

- `--dep`: Switch `tree` from physical filesystem tree to logical import dependency tree.
- `--loc`: Display physical SLOC on tree nodes in an aligned column.
- `--outline` / `--symbols`: Display defined structs, classes, and traits. (In `tree`: on the same line in a right-hand column; in `loc`: nested under each file).
- `--flat`: Flatten `loc` view into a single global table sorted by SLOC.
- `--no-ignore`: Ignore `.gitignore` rules and scan everything.
- `-l, --lang <LANG>`: Filter by language (`c`, `cpp`, `py`, `js`, `ts`, `lua`, `luau`, `rs`).
- `-e, --ext <EXTS...>`: Filter by extension (e.g. `-e ts,tsx`).
- `-d, --depth <N>`: Maximum recursion depth.
- `--color-distinct`: Enable colorblind-safe high-contrast distinct color assignment.
- `--raw`: Emit clean, unformatted, pipeable text (one item per line) for consumption by `todo`, `palmtree`, or scripts.
- `--json`: Emit structured JSON for MCP server or automation tools.
- `--no-color`: Disable ANSI color escape codes.

---

## 7. Compiler-Style Diagnostics (`spaghetti diag`)

Diagnostics use a compiler-style `-W` flag system. Opinionated metrics like indentation depth are omitted; focus is on actionable architectural debt.

### Warning Flags & Defaults

| Flag | Name | Default | Threshold / Description |
| :--- | :--- | :---: | :--- |
| `-Worphan` | Dead / Unimported Files | **Enabled** | Warns on internal source files with 0 incoming imports that are not entry points. |
| `-Wcycle` | Circular Dependencies | **Enabled** | Warns on circular import loops ($A \to B \to A$). |
| `-Wgod` | God Modules | **Enabled** | Warns on files exceeding a SLOC threshold (Default: `1000` SLOC). |
| `-Wdoc` | Under-documented Files | *Disabled* | Warns when a file has less than target comment ratio (Default: `< 5%` comments). |
| `-Wall` | All Warnings | — | Enables all available diagnostic warnings (`-Worphan`, `-Wcycle`, `-Wgod`, `-Wdoc`). |
| `-Wno-<tag>`| Suppress Warning | — | Disables a specific warning (e.g., `-Wno-god`, `-Wno-orphan`). |
| `-Werror` | Strict Exit | — | Treats warnings as errors (exits with non-zero exit code). |

### Custom Threshold Flags
- `--god-loc <N>` (or `-Wgod=<N>`): Customizes the SLOC threshold for god files (e.g., `--god-loc 1200`).
- `--doc-ratio <PERCENT>`: Customizes under-documented threshold (e.g., `--doc-ratio 10`).

---

## 8. Console Output Formats & Visual Mockups

### A. Filesystem Code Tree (`spaghetti tree`)
Filters out `.json`, `.csv`, `.lock`, `.md`, and respects `.gitignore`:
```text
$ spaghetti tree

my_project/
├── include/
│   └── parser.h
├── src/
│   ├── engine/
│   │   ├── core.cpp
│   │   └── physics.cpp
│   └── main.cpp
└── scripts/
    └── deploy.py

5 code files found (4 C++, 1 Python). Ignored 9 non-code files (.json, .lock, .md).
```

### B. Filesystem Tree with Same-Line Columns (`spaghetti tree --loc --outline`)
Line counts and extracted structs/classes appear on the **same line in dedicated columns**:
```text
$ spaghetti tree --loc --outline

my_project/
├── include/
│   └── parser.h              [ 120 loc]  struct Token, class Parser
├── src/
│   ├── engine/
│   │   ├── core.cpp          [1,425 loc]  class EngineCore, struct EngineState
│   │   └── physics.cpp       [  680 loc]  class RigidBody, struct Vector3
│   └── main.cpp              [   95 loc]  fn main
└── scripts/
    └── deploy.py             [   69 loc]  class DeployConfig

Total: 5 code files | 2,389 SLOC | 6 types defined
```

### C. Logical Dependency Tree (`spaghetti tree --dep --loc --outline`)
Same column alignment applied to the logical import graph:
```text
$ spaghetti tree --dep --loc --outline

Project Dependency Map (Root: src/main.cpp)
src/main.cpp                  [   95 loc]  fn main
├── include/parser.h          [ 120 loc]  struct Token, class Parser
└── src/engine/core.cpp       [1,425 loc]  class EngineCore, struct EngineState
    ├── src/engine/physics.cpp[  680 loc]  class RigidBody, struct Vector3
    └── include/parser.h      (dedup ↷)

Found 1 root entrypoint, 4 files in dependency graph.
```

If a circular import occurs:
```text
└── src/auth.rs               [  320 loc]  struct Session
    └── src/user.rs           [  280 loc]  struct UserProfile
        └── src/auth.rs       (cycle ⟲) [WARNING: -Wcycle]
```

### D. Language-Grouped LOC Ranking (`spaghetti loc`)
Sorted from most to least code within each language:
```text
$ spaghetti loc

C/C++ (3 files, 2,225 loc)
    src/engine/core.cpp          1,425 loc
    src/engine/physics.cpp         680 loc
    include/parser.h               120 loc

RUST (2 files, 415 loc)
    src/auth.rs                    320 loc
    src/main.rs                     95 loc

PYTHON (1 file, 69 loc)
    scripts/deploy.py               69 loc

Total: 6 code files | 2,709 SLOC
```

### E. LOC Ranking with Symbols (`spaghetti loc --outline`)
Nests defined classes and structs directly under each ranked file:
```text
$ spaghetti loc --outline

C/C++ (3 files, 2,225 loc)
    src/engine/core.cpp          1,425 loc
        • [class]  EngineCore
        • [struct] EngineState
    src/engine/physics.cpp         680 loc
        • [class]  RigidBody
        • [struct] Vector3
    include/parser.h               120 loc
        • [class]  Parser
        • [struct] Token

PYTHON (1 file, 69 loc)
    scripts/deploy.py               69 loc
        • [class]  DeployConfig

Total: 4 files | 7 types defined
```

### F. Flat Global LOC Ranking (`spaghetti loc --flat`)
```text
$ spaghetti loc --flat --limit 5

RANK   FILE                         LANG      SLOC
  1.   src/engine/core.cpp          C++      1,425
  2.   src/engine/physics.cpp       C++        680
  3.   src/auth.rs                  Rust       320
  4.   include/parser.h             C++        120
  5.   src/main.rs                  Rust        95
```

### G. Diagnostics Output (`spaghetti diag`)
```text
$ spaghetti diag -Wall --god-loc 1000

[spaghetti diagnostics] Scanning 42 code files...

[-Wcycle] Circular Dependencies:
  ! src/auth.rs <-> src/user.rs (2-file cycle)

[-Worphan] Dead / Unimported Code Files:
  ? src/legacy/old_parser.c (0 incoming imports, not an entrypoint)
  ? scripts/dead_util.py   (0 incoming imports)

[-Wgod] God Files (> 1000 SLOC):
  ! src/engine/core.cpp (1,425 SLOC, 18 imports)

[-Wdoc] Under-documented Files (< 5% comments):
  ! src/net/socket.rs (380 SLOC, 3 comment lines [0.7%])

Found 5 warnings across 4 categories (use -Wno-<tag> to suppress).
```

---

## 9. UNIX Composable Piping Mechanics

In accordance with the workspace's UNIX philosophy, `spaghetti` supports non-blocking raw streaming:

### 1. Piped into `todo`
Auto-convert dead code diagnostics or circular dependency warnings into tasks:
```bash
# Finds dead orphan files and pipes them directly into todo tasks:
spaghetti diag -Worphan --raw | while read -r file; do todo n "Review dead file $file" tech-debt; done

# Or pipe god files into todo:
spaghetti diag -Wgod --raw | while read -r file; do todo n "Refactor god-module $file" refactor; done
```

### 2. Piped into `palmtree`
Start a Pomodoro sprint directly on the largest module:
```bash
# Pick the largest file across the repo and feed it into palmtree:
spaghetti loc --flat --raw --limit 1 | palmtree start
```

---

## 10. MCP Integration: Bridging to `bake-mcp`

`spaghetti` will integrate with the existing `bake-mcp` server daemon (defined in `rust_practice/MCP_PLAN.md`).

### Exposed MCP Tools

| MCP Tool Name | Arguments | Output Description |
| :--- | :--- | :--- |
| `spaghetti_tree` | `{ path?: string, dep?: bool, loc?: bool, outline?: bool }` | Structured JSON tree (filesystem or dependency) with same-line metadata. |
| `spaghetti_loc` | `{ path?: string, flat?: bool, outline?: bool, lang?: string }` | Language-grouped or flat SLOC ranking with optional symbol lists. |
| `spaghetti_diagnostics`| `{ wall?: bool, god_loc?: number, doc_ratio?: number }` | Active warnings (`-Worphan`, `-Wcycle`, `-Wgod`, `-Wdoc`). |

---

## 11. Future Feature Roadmap

1. **Mermaid Exporter (`spaghetti tree --dep --mermaid`):**
   Generates native GitHub Markdown flowchart diagrams from dependency trees.
2. **Git Impact / Blast Radius (`spaghetti impact [branch/commit]`):**
   Computes all files affected upstream by incoming Git diffs.
3. **Architecture Watch Mode (`spaghetti watch`):**
   Lightweight file watcher alerting in terminal whenever a new circular dependency or god file is introduced during editing.
