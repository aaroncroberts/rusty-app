# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

rusty-app is a universal database tool written in Rust using GPUI. The goal is to create a powerful, cross-platform database management application that supports multiple database systems with a modern, performant UI. In spirit similar to universal database tools like DBeaver, but with its own approach and identity.

## System Requirements

### All Platforms
- Rust toolchain (stable)
- Platform-specific graphics drivers for GPU acceleration (Iced uses wgpu)

## Common Commands

```bash
# Build the project
cargo build

# Run the application
cargo run

# Check for errors without building
cargo check

# Run linter
cargo clippy

# Format code
cargo fmt

# Run tests
cargo test
```

## Architecture

### UI Framework: Iced

The application uses Iced, a cross-platform GUI library which provides:
- GPU-accelerated rendering via wgpu for high performance
- Elm-inspired architecture with declarative UI
- Clean, modern aesthetic suitable for polished applications
- Native feel with cross-platform support (Windows, macOS, Linux)

### Target Database Systems

Priority support for:
- **PostgreSQL** - Full-featured relational database
- **MySQL/MariaDB** - Popular open-source RDBMS
- **SQLite** - Embedded database for local files
- **MongoDB** - Document-oriented NoSQL
- **Redis** - In-memory data structure store
- **SQL Server** - Microsoft's enterprise database (future)

### Core Modules (Planned Architecture)

- **Connection Manager** - Handle database connections, credential storage, connection pooling
- **Query Editor** - SQL/query editor with syntax highlighting, autocomplete, execution
- **Result Viewer** - Display query results in grid/table format with pagination
- **Schema Browser** - Tree view of databases, tables, columns, indexes, relationships
- **Data Editor** - In-place editing of table data
- **Query History** - Track and replay previous queries
- **Export/Import** - Data migration tools for various formats (CSV, JSON, SQL)

### Layout Structure (Planned)

```
┌─────────────────────────────────────────────┐
│ Menu Bar & Toolbar                          │
├──────────┬──────────────────┬───────────────┤
│          │                  │               │
│ Database │  Query Editor    │  Auxiliary    │
│ Navigator│  or              │  Panel        │
│ (Tree)   │  Result Grid     │               │
│          │                  │               │
└──────────┴──────────────────┴───────────────┘
```

### Key Design Principles

1. **Async-First** - All database operations use `tokio` for non-blocking I/O
2. **Connection Pooling** - Efficient connection management per database
3. **Plugin Architecture** - Each database driver is a separate module with a common trait
4. **Result Streaming** - Handle large result sets without loading everything into memory
5. **Cross-Platform** - Support macOS, Linux, and Windows

## Development Workflow

### Adding a New Database Driver

1. Create module in `src/drivers/` (e.g., `postgres.rs`)
2. Implement the `DatabaseDriver` trait
3. Add connection string parsing
4. Implement query execution and result mapping
5. Add schema introspection queries

### GPUI Component Pattern

Components in GPUI:
- Define state using structs
- Implement rendering via the `Render` trait
- Use actions for user interactions
- Leverage reactive updates when state changes

## Dependencies (Current/Planned)

- **iced** - Cross-platform GUI framework with GPU acceleration
- **tokio** - Async runtime for database operations
- **serde / serde_json** - Serialization for configuration files
- **mongodb** - MongoDB Rust driver (planned)
- **sqlx** - SQL database driver with compile-time query checking (planned)
- **redis** - Redis client (planned)
