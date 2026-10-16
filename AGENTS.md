# CONTEXT.md — Project & Architectural Overview

## Project Overview
`rust_practice` is a monorepo containing fast, zero-bloat command-line utilities written in Rust. The workspace is built around the UNIX philosophy: small, single-purpose terminal tools that run with zero latency and pipe into one another seamlessly.

### Core Tools
1. **`todo` (`todoCli/`)**: A lightweight task manager designed for fast keyboard entry.
   - Saves state to `my_todo.json`.
   - Supports quote-less task creation (`todo n buy milk`).
   - Supports task tagging, attaching files, searching, and file scanning to find "// TODO:" comments.
2. **`palmtree` (`palmtree/`)**: A customizable Pomodoro timer for terminal-driven workflows.
   - Features work/break cycles, pause tracking, system notifications, and preset productivity ratios (e.g., standard Pomodoro, DeskTime, Flow).
   - Designed to read piped task inputs directly from `todo`.

---

## Core Philosophy & Development Goals
- **Zero Bloat:** Minimal dependencies, tiny binary sizes, near-zero startup time, and no unnecessary abstractions.
- **Manual Rust Mastery:** The primary author builds and refactors code manually to master idiomatic Rust, using AI strictly for targeted debugging, documentation, or architectural review.
- **Terminal Reliability:** High emphasis on minimal but clean TUI rendering—preventing layout twitching, terminal wrapping bugs, or stdin blocking when piped.

---

## Workspace & Data Structure

```text
rust_practice/
├── todoCli/
│   ├── src/
│   │   │── main.rs         # main functionality: task management, tags/files, searching, etc.
│   │   │── core.rs         # JSON persistence and task print formatting
│   │   └── scanner.rs      # handles scanning files for todo comments
│   └── Cargo.toml
├── pomodoro/
│   ├── src/
│   │   └── main.rs         # Timer loop, crossterm UI, notification alerts
│   └── Cargo.toml
└── README.md
```

### Persistence

* **`todo`**: Persists task lists to `my_todo.json` in the working directory (or user data directory).
* **`palmtree`**: Persists user preferences to `~/.config/palmtree/config.json`.

---

## Technical Architecture & Important Mechanics

### 1. `todo` Piping Mechanics

* **Quoteless Input:** Command-line args after subcommands (`todo n ...`) are joined into a single string so quotes aren't required.
* **Compact Print:** Outputs *only* the raw task name (but has flags for completion status, and everything else) to `stdout` without decorative UI, enabling piping:
```bash
todo print 0 | palmtree start
```



### 2. `palmtree` TUI & Lifecycle (`main.rs`)

#### **Raw Mode Guard (`RawModeGuard`)**

RAII guard handling terminal raw mode and cursor visibility.

* **Terminal Awareness:** Checks `io::stdout().is_terminal()` and `io::stdin().is_terminal()`.
* **Piping Safety:** Ensures raw mode is only toggled on interactive standard streams so piped standard inputs (from `todo`) don't crash terminal state.

#### **Non-Blocking Piped Stdin (`get_piped_task`)**

* Inspects `io::stdin()`. If `is_terminal()` is `false`, reads the first line from the buffer into a `Task` string and passes it into the timer session.

#### **TUI Rendering (`run_timer`)**

* Uses `crossterm` for low-level screen updates.
* **Line-by-Line Clears:** Executes `MoveUp(3)` and explicitly clears each line (`Clear(ClearType::UntilNewLine)`) followed by `writeln!` to maintain exact 1:1 row replacement and eliminate terminal flickering/scrolling drift.
* **String Truncation:** Task output is bounded using format specifiers (e.g., `{:.60}`) to prevent text wrapping beyond terminal width, which would break line-height arithmetic.

---

## Instructions for AI Assistants

When assisting with code modifications or features in this repository:

1. **Maintain Zero Bloat:** Avoid introducing heavy dependencies (e.g., complex TUI frameworks like `ratatui` unless explicitly requested). Stick to light crates like `crossterm`, `serde`, and `notify-rust`.
2. **Preserve Piping Compatibility:** Any UI or input change to `todo` or `palmtree` must preserve non-interactive stdin/stdout piping.
3. **Respect Line Arithmetic:** When tweaking terminal output in `palmtree`, ensure every line move up matches the exact count of newlines emitted on render.
4. **Idiomatic Rust:** Write clean, safe Rust focusing on clear error handling (`Result`/`Option`) without excessive panic states.
