# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

rusty-app is a universal database tool written in Rust using [Iced](https://github.com/iced-rs/iced). The goal is a powerful, cross-platform database management application supporting multiple database systems with a modern, performant UI — similar in spirit to DBeaver but with its own identity.

## Common Commands

```bash
# Build and run
cargo run

# Check for errors (fast)
cargo check

# Lint (CI-equivalent — all warnings are errors)
cargo clippy --features postgres,mysql,sqlite,mongodb,mssql -- -D warnings

# Format
cargo fmt

# Test
cargo test --features postgres,mysql,sqlite,mongodb,mssql

# Full local CI check (mirrors GitHub Actions exactly)
bash scripts/ci-check.sh
```

> **Note**: Oracle requires native Oracle Instant Client libraries. Omit `oracle` from `--features` if not installed. The `default` feature set includes oracle; CI excludes it.

## Architecture

### UI Framework: Iced

- GPU-accelerated rendering via wgpu
- Elm-inspired architecture: `Message` enum → `update()` → `view()`
- Declarative, stateless view functions
- Cross-platform: macOS, Linux, Windows

### Database Access: arni

All database I/O goes through the [arni](https://github.com/aaroncroberts/arni) library via the `DbAdapter` trait. rusty-app never talks to databases directly.

- `connection_manager.rs` — owns `ActiveConnection` (wraps `Arc<Mutex<Box<dyn DbAdapter>>>`)
- `make_adapter()` — feature-gated factory dispatching to arni's per-database adapters
- Supported: PostgreSQL, MySQL, SQLite, MongoDB, SQL Server, Oracle

### Module Map

| Module | Purpose |
|---|---|
| `main.rs` | App entry point, `DatabaseIDE` state, `Message` enum, `update()`, `view()` |
| `connection_manager.rs` | Active connections, `ActiveConnection`, `ConnectionManager` |
| `left_panel.rs` | Collapsible left panel with tab bar (28px icon rail when collapsed) |
| `components/` | Reusable UI components: `ServerListComponent`, `TableListComponent`, `PropertiesComponent`, `ConnectionFormComponent` |
| `icons.rs` | Nerd Font (Codicons) icon helpers — all icon functions return `String` glyphs |
| `theme.rs` | `ThemeColors` struct — Tokyo Night Storm palette |
| `button_styles.rs` | Shared button style helpers (`primary`, `secondary`, `danger`, `disabled`) |
| `settings/` | User settings, persistence, `SettingsManager` |
| `container/` | Podman container management for local dev databases |
| `views.rs` | `ViewRegistry` — maps `RegionId` to enabled components |

### Layout

```
┌──────────────────────────────────────────┐
│ Menu Bar                                 │
├──────────┬───────────────────────────────┤
│          │                               │
│ Left     │  Main Panel                  │
│ Panel    │  (Query Editor / Results)    │
│ (tabs)   │                               │
│          │                               │
└──────────┴───────────────────────────────┘
│ Status Bar                               │
└──────────────────────────────────────────┘
```

The left panel collapses to a 28px icon rail (`left_panel::ICON_RAIL_WIDTH`). Toggle via `Message::ToggleLeftPanel`.

### Icons

```rust
use rusty_app::icons;
text(icons::database()).font(icons::font()).size(14).color(theme.accent)
```

Font must be registered at app startup via `.font(icons::FONT_BYTES)` on the iced application builder (already wired in `main.rs`).

### Key Design Principles

1. **Async-First** — All database operations are `async`, driven by `tokio`
2. **Adapter trait** — `DbAdapter` from arni; swap databases without changing UI code
3. **Feature flags** — Each database adapter is a Cargo feature; oracle excluded from CI
4. **No dead code in CI** — `clippy -D warnings` is enforced; all warnings are errors

## CI

GitHub Actions runs on every push and PR to `main`:
- `cargo fmt --check`
- `cargo check`
- `cargo clippy -- -D warnings`
- `cargo test`

Oracle excluded from CI (requires native OCI libs not available on runners).
