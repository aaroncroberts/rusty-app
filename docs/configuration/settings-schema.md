# Settings File Schema Documentation

## Overview

rusty-app uses a centralized TOML configuration file located at `~/.rusty-app/settings.toml`. This file controls logging behavior, database connections, and UI preferences.

**Location:** `~/.rusty-app/settings.toml`
**Format:** TOML (Tom's Obvious, Minimal Language)
**Example:** See [`settings.toml.example`](../../settings.toml.example) in the repository root

## File Structure

The settings file is organized into three main sections:

1. **[logging]** - Logging configuration (integrates with rusty-logging)
2. **[[connections]]** - Database connection definitions (array)
3. **[ui_preferences]** - User interface settings

## Logging Configuration

The `[logging]` section configures the rusty-logging crate, which provides structured logging for both rusty-app and rusty-data.

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `filter` | string | `"info"` | Global log level filter |
| `console_filter` | string (optional) | (uses `filter`) | Console-specific log level (overrides global) |
| `console_format` | string | `"compact"` | Console output format: `"pretty"` or `"compact"` |
| `console_writer` | string | `"stderr"` | Console output stream: `"stdout"` or `"stderr"` |
| `console_enabled` | boolean | `true` | Enable/disable console logging |
| `file_filter` | string (optional) | (uses `filter`) | File-specific log level (overrides global) |
| `file_enabled` | boolean | `true` | Enable/disable file logging |
| `file_format` | string | `"text"` | File output format: `"text"` or `"json"` |
| `file_directory` | string | `"~/.rusty-app/logs"` | Directory for log files |
| `file_prefix` | string | `"rusty-app"` | Prefix for log file names |
| `rotation_policy` | string | `"daily"` | File rotation: `"daily"`, `"hourly"`, `"minutely"`, `"never"` |

### Log Level Filters

Log levels (from most to least verbose):
- `trace` - Very detailed debugging information
- `debug` - Debugging information
- `info` - General informational messages (default)
- `warn` - Warning messages
- `error` - Error messages only

**Module-Specific Filtering:**

You can target specific modules with different log levels:

```toml
[logging]
filter = "info,rusty_app=debug,rusty_data::adapters=trace"
```

This sets the global level to `info`, but enables `debug` for rusty_app and `trace` for rusty_data's adapter modules.

### Console Formats

**Pretty Format** (`console_format = "pretty"`):
- Colorized output for terminal viewing
- Full timestamps and metadata
- Best for development and debugging
- Example: `2024-01-15 14:30:22.123 DEBUG rusty_app: Connection established`

**Compact Format** (`console_format = "compact"`):
- Minimal, single-line output
- Reduced metadata for production logs
- Example: `[14:30:22] DEBUG Connection established`

### File Formats

**Text Format** (`file_format = "text"`):
- Human-readable `.log` files
- Good for manual inspection
- Example: `2024-01-15 14:30:22.123 INFO rusty_app::database - Query executed in 45ms`

**JSON Format** (`file_format = "json"`):
- Structured JSON Lines (`.jsonl`) files
- Each line is a complete JSON object
- Best for log aggregation and analysis tools
- Example: `{"timestamp":"2024-01-15T14:30:22.123Z","level":"INFO","target":"rusty_app::database","message":"Query executed in 45ms"}`

### File Rotation Policies

| Policy | Behavior | File Name Pattern |
|--------|----------|-------------------|
| `daily` | New file at midnight | `rusty-app_2024-01-15.log` |
| `hourly` | New file every hour | `rusty-app_2024-01-15-14.log` |
| `minutely` | New file every minute (testing) | `rusty-app_2024-01-15-14-30.log` |
| `never` | Single file, no rotation | `rusty-app.log` |

### Example: Development Logging

```toml
[logging]
filter = "debug"
console_format = "pretty"
console_writer = "stdout"
console_enabled = true
file_enabled = true
file_format = "text"
file_directory = "~/.rusty-app/logs"
file_prefix = "rusty-app"
rotation_policy = "daily"
```

### Example: Production Logging

```toml
[logging]
filter = "info"
console_filter = "warn"  # Only warnings and errors to console
console_format = "compact"
console_writer = "stderr"
console_enabled = true
file_filter = "info"  # Full info logs to file
file_enabled = true
file_format = "json"  # Structured logs for analysis
file_directory = "/var/log/rusty-app"
file_prefix = "rusty-app"
rotation_policy = "daily"
```

## Database Connections

The `[[connections]]` array defines database connections. Each entry is a separate connection configuration.

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier (alphanumeric, dashes, underscores only) |
| `name` | string | Display name shown in the UI |
| `db_type` | string | Database type (see Database Types below) |
| `database` | string | Database name or file path (for SQLite) |

### Optional Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | string | - | Server hostname (not used for SQLite) |
| `port` | integer | (db default) | Server port number |
| `username` | string | - | Authentication username |
| `use_ssl` | boolean | `false` | Enable SSL/TLS encryption |
| `parameters` | table | `{}` | Additional connection parameters |

### Database Types

| `db_type` | Description | Default Port | Notes |
|-----------|-------------|--------------|-------|
| `postgres` | PostgreSQL | 5432 | Fully supported |
| `mysql` | MySQL/MariaDB | 3306 | Fully supported |
| `sqlite` | SQLite | - | File-based, no host/port |
| `mongodb` | MongoDB | 27017 | Fully supported |
| `sqlserver` | Microsoft SQL Server | 1433 | Fully supported |
| `oracle` | Oracle Database | 1521 | Uses SID for `database` field |

### Connection ID Rules

- Must be unique across all connections
- Alphanumeric characters, dashes, and underscores only
- No spaces
- Case-sensitive
- Reserved prefix: `local-` (used for container-managed connections)

**Valid IDs:** `prod-db`, `staging_mysql`, `dev_postgres_01`
**Invalid IDs:** `prod db` (space), `prod.db` (period), `123` (too generic)

### Connection Parameters

Additional connection parameters can be specified in a `[connections.parameters]` table:

```toml
[[connections]]
id = "secure-postgres"
name = "Secure PostgreSQL"
db_type = "postgres"
host = "db.example.com"
port = 5432
database = "prod_db"
username = "app_user"
use_ssl = true

[connections.parameters]
sslmode = "require"
connect_timeout = "10"
application_name = "rusty-app"
```

### Example Connections

**PostgreSQL (Local Development):**
```toml
[[connections]]
id = "local-postgres"
name = "Local PostgreSQL (Container)"
db_type = "postgres"
host = "localhost"
port = 5432
database = "test_db"
username = "test_user"
use_ssl = false
```

**MySQL (Production):**
```toml
[[connections]]
id = "prod-mysql"
name = "Production MySQL"
db_type = "mysql"
host = "mysql.prod.example.com"
port = 3306
database = "app_database"
username = "app_user"
use_ssl = true
```

**SQLite (File-based):**
```toml
[[connections]]
id = "local-sqlite"
name = "Local Project Database"
db_type = "sqlite"
database = "/Users/me/projects/myapp/data.db"
# host, port, username not used for SQLite
use_ssl = false
```

**SQL Server (Container):**
```toml
[[connections]]
id = "local-mssql"
name = "Local SQL Server (Container)"
db_type = "sqlserver"
host = "localhost"
port = 1433
database = "master"
username = "sa"
use_ssl = false
```

**Oracle (Container):**
```toml
[[connections]]
id = "local-oracle"
name = "Local Oracle (Container)"
db_type = "oracle"
host = "localhost"
port = 1521
database = "XE"  # SID, not database name
username = "test_user"
use_ssl = false
```

## UI Preferences

The `[ui_preferences]` section stores user interface settings.

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `theme` | string | `"dark"` | UI theme: `"dark"` or `"light"` |
| `panel_width` | integer | `250` | Left panel width in pixels (200-600) |
| `show_left_panel` | boolean | `true` | Show left panel on startup |
| `active_component` | string | `"server_list"` | Active component on startup |
| `window_width` | integer | `1280` | Window width in pixels |
| `window_height` | integer | `800` | Window height in pixels |

### Active Component Options

- `"server_list"` - Server/connection list
- `"table_list"` - Table browser
- `"properties"` - Properties panel

### Example

```toml
[ui_preferences]
theme = "dark"
panel_width = 300
show_left_panel = true
active_component = "server_list"
window_width = 1600
window_height = 1000
```

## File Location and Initialization

### Default Location

`~/.rusty-app/settings.toml`

- `~` expands to the user's home directory
- Directory is created automatically if missing
- On Linux/macOS: `/home/username/.rusty-app/settings.toml`
- On Windows: `C:\Users\Username\.rusty-app\settings.toml`

### Initialization Behavior

1. **File exists:** Load settings from `~/.rusty-app/settings.toml`
2. **File missing:** Create default settings file with sensible defaults
3. **Invalid TOML:** Report error and fall back to defaults
4. **Missing fields:** Use built-in defaults for missing values

### Migration from connections.toml

When migrating from the old `~/.rusty-app/connections.toml` format:

1. Old connections are automatically imported into `settings.toml`
2. Original `connections.toml` is backed up as `connections.toml.backup`
3. Migration happens once on first startup with new settings system

## Validation Rules

### Logging Validation

- `filter`, `console_filter`, `file_filter` must be valid EnvFilter syntax
- `console_format` must be `"pretty"` or `"compact"`
- `console_writer` must be `"stdout"` or `"stderr"`
- `file_format` must be `"text"` or `"json"`
- `rotation_policy` must be `"daily"`, `"hourly"`, `"minutely"`, or `"never"`
- `file_directory` must be a valid path
- `file_prefix` must not be empty

### Connection Validation

- `id` must be unique and non-empty
- `id` must match regex: `^[a-zA-Z0-9_-]+$`
- `db_type` must be a supported database type
- `database` must not be empty
- For non-SQLite: `host` is required
- For non-SQLite: `port` is required (or uses default)
- `port` must be between 1 and 65535 if specified

### UI Preferences Validation

- `theme` must be `"dark"` or `"light"`
- `panel_width` must be between 200 and 600
- `active_component` must be a valid component name
- `window_width` and `window_height` must be positive integers

## Reloading Settings

Settings can be reloaded without restarting the application:

1. **Via UI:** File > Reload Settings (or View > Settings > Reload button)
2. **Programmatically:** `SettingsManager::reload()`

### Reload Behavior

- **Logging settings:** Reconfigures rusty-logging immediately
- **Connection changes:** Disconnects old connections, loads new ones
- **UI preferences:** Updates UI state (theme, panel width, etc.)

### Atomic Saves

Settings are saved atomically to prevent corruption:

1. Write to temporary file: `~/.rusty-app/settings.toml.tmp`
2. Validate written TOML
3. Rename temporary file to `settings.toml` (atomic operation)

This ensures settings are never partially written or corrupted.

## Best Practices

### Security

- **Never commit** `settings.toml` to version control (contains connection credentials)
- Use environment-specific settings files for different environments
- Consider encrypting sensitive connection credentials (future feature)
- Use SSL/TLS (`use_ssl = true`) for production database connections

### Organization

- Group connections by environment (prefix with `local-`, `dev-`, `prod-`)
- Use descriptive connection names that indicate purpose and environment
- Document custom connection parameters with inline comments
- Keep log files in a dedicated directory separate from application data

### Performance

- Use `compact` console format for production (less overhead)
- Set appropriate log levels (`info` or `warn` for production, `debug` for development)
- Use `daily` rotation policy for most applications (hourly for high-volume)
- Consider `json` file format if logs will be analyzed programmatically

### Maintainability

- Keep the example file (`settings.toml.example`) up to date
- Document any custom connection parameters
- Use consistent naming conventions for connection IDs
- Validate settings after manual edits before reloading

## Troubleshooting

### Settings Not Loading

**Symptom:** Application uses defaults instead of settings file

**Causes:**
- File doesn't exist at `~/.rusty-app/settings.toml`
- Invalid TOML syntax
- File permissions prevent reading

**Solutions:**
1. Check file exists: `ls -la ~/.rusty-app/settings.toml`
2. Validate TOML: Use an online TOML validator
3. Check permissions: `chmod 644 ~/.rusty-app/settings.toml`
4. Review application logs for error messages

### Invalid Log Filter

**Symptom:** Error message about invalid filter

**Cause:** `filter`, `console_filter`, or `file_filter` has invalid syntax

**Solutions:**
- Use simple log levels: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`
- For module targeting: `"info,module::path=debug"`
- Check for typos in module names

### Connection Fails to Load

**Symptom:** Connection appears in settings but not in UI

**Causes:**
- Missing required fields (`id`, `name`, `db_type`, `database`)
- Invalid `db_type` value
- Duplicate `id`
- Validation failure

**Solutions:**
1. Check all required fields are present
2. Verify `db_type` is one of: `postgres`, `mysql`, `sqlite`, `mongodb`, `sqlserver`, `oracle`
3. Ensure `id` is unique across all connections
4. Review application logs for validation errors

## Schema Evolution

Future versions may add new fields. The schema is designed for forward compatibility:

- New fields will have sensible defaults
- Missing fields will not cause errors
- Unknown fields will be ignored (with a warning)
- Settings from older versions will be automatically upgraded

## Related Documentation

- [rusty-logging Configuration](../logging/configuration.md) - Detailed logging setup
- [Database Connections](../database/connections.md) - Connection management guide
- [Container Databases](../development/container-analysis.md) - Local development containers
- [UI Customization](../ui/preferences.md) - UI preferences guide
