# spaghetti 🍝

Fast, zero-bloat codebase cartographer, dependency tree mapper, and architectural onboarding tool for the terminal.

Part of the [`bake`](../) lightweight CLI productivity suite alongside `todo` and `palmtree`.

---

## Documentation

- **[Architecture & Implementation Plan (PLAN.md)](PLAN.md)**: Detailed technical specifications, multi-language import scanner, graph cycle-breaking algorithms (Tarjan's SCC & 3-color DFS), CLI syntax, diagnostics metrics, and MCP integration.

## Key Capabilities

- **Zero-Bloat Startup:** Instantaneous execution with minimal dependencies (`crossterm`, `serde`).
- **Multi-Language Import Scanning:** C/C++, Python, JavaScript/TypeScript (.ts, .mjs), Lua/Luau, and Rust.
- **Cycle-Resilient Trees:** Automatically discovers canonical entrypoint roots and handles circular dependencies with cycle back-edge annotations.
- **Onboarding Symbol Inspector:** Rapidly extracts structs, classes, interfaces, traits, and functions so newcomers understand where things are defined.
- **UNIX Pipeable:** Clean `--raw` text output designed to pipe directly into `todo` and `palmtree`, plus structured `--json` for AI agents via `bake-mcp`.
