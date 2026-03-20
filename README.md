# rusty-app

Universal database management tool written in Rust using [Iced](https://github.com/iced-rs/iced).

## Overview

rusty-app is a cross-platform desktop application for managing multiple database systems. It provides a modern, GPU-accelerated UI built with Iced and delegates all database access to the [arni](https://github.com/aaroncroberts/arni) library.

## Project Structure

```
rusty-app/
├── rusty-app/          # UI application crate
│   └── src/
│       ├── components/ # Reusable UI components (ServerList, TableList, Properties, ...)
│       ├── container/  # Podman container management for local dev databases
│       ├── settings/   # Configuration and user preferences
│       ├── icons.rs    # Nerd Font (Codicons) icon helpers
│       ├── theme.rs    # Color theme (dark mode)
│       └── main.rs     # Application entry point
├── rusty-logging/      # Structured logging configuration crate
├── scripts/
│   └── ci-check.sh     # Local CI verification script
└── docs/               # Documentation
```

## Supported Databases

| Database | Status |
|---|---|
| PostgreSQL | ✅ Supported |
| MySQL / MariaDB | ✅ Supported |
| SQLite | ✅ Supported |
| MongoDB | ✅ Supported |
| SQL Server | ✅ Supported |
| Oracle | ✅ Supported (requires Oracle Instant Client) |

## Getting Started

### Prerequisites

- Rust stable toolchain
- macOS: Xcode Command Line Tools

### Build and Run

```bash
# Build and run (all adapters including Oracle)
cargo run

# Build without Oracle (no Instant Client required)
cargo run --no-default-features --features postgres,mysql,sqlite,mongodb,mssql

# Run tests
cargo test --features postgres,mysql,sqlite,mongodb,mssql
```

### Local CI Check

Mirrors the GitHub Actions CI pipeline exactly:

```bash
bash scripts/ci-check.sh
```

Runs: `cargo fmt --check` → `cargo check` → `cargo clippy -D warnings` → `cargo test`.

## Documentation

- **[CLAUDE.md](CLAUDE.md)** — Architecture notes and development guidelines
- **[docs/](docs/)** — Additional documentation

## License

MIT
