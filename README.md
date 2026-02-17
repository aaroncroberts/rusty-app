# rusty-app

Universal database management tool written in Rust with GPUI.

## Overview

rusty-app is a cross-platform database tool that provides a modern interface for managing multiple database systems. Built with Rust and GPUI for performance and a native feel.

## Project Structure

```
rusty-app/
├── rusty-app/          # UI application crate
├── rusty-data/         # Data access library crate
│   ├── src/
│   │   ├── adapters/   # Database adapters (PostgreSQL, MySQL, SQLite)
│   │   ├── adapter.rs  # DatabaseAdapter trait
│   │   ├── config.rs   # Configuration management
│   │   └── error.rs    # Error types
│   └── podman-compose.yml  # Database testing containers
└── docs/               # Documentation
    ├── README.md       # Documentation overview
    └── database/       # Database-specific docs
        └── testing.md  # Testing environment setup
```

## Supported Databases

- **PostgreSQL** - Full-featured relational database
- **MySQL/MariaDB** - Popular open-source RDBMS
- **SQLite** - Embedded database for local files
- **MongoDB** - Document-oriented NoSQL (planned)
- **SQL Server** - Microsoft's enterprise database (planned)

## Getting Started

### Prerequisites

- Rust (latest stable)
- Xcode Command Line Tools (macOS)
- Metal Toolchain (macOS, for GPUI GPU acceleration)
- Podman (for database testing containers)

### Build and Run

```bash
# Build the project
cargo build

# Run the application
cargo run

# Run tests
cargo test

# Run tests with database features
cargo test --features all-databases
```

### Database Testing Environment

See [Database Testing Documentation](docs/database/testing.md) for setting up local database containers using podman-compose.

## Documentation

- **[Documentation Index](docs/README.md)** - Complete documentation overview
- **[Database Testing](docs/database/testing.md)** - Setup testing databases
- **[CLAUDE.md](CLAUDE.md)** - Guidelines for AI assistants working with this codebase

## License

MIT
