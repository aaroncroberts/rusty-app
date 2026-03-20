# Container Database Analysis for rusty-app UI Integration

## Overview

This document provides a comprehensive analysis of the `icitadel-dev-*` container databases used by rusty-data tests, enabling their integration into the rusty-app UI for local development support.

**Source Files:**
- `rusty-data/podman-compose.yml` - Container definitions
- `docs/database/testing.md` - Container usage documentation

## Container Inventory

All containers follow the naming pattern: `icitadel-dev-{platform}`

### 1. icitadel-dev-postgres (PostgreSQL 16)

**Image:** `postgres:16-alpine`
**Port:** 5432 (standard)
**Credentials:**
- User: `test_user`
- Password: `test_password`
- Database: `test_db`

**Connection String Pattern:**
```
host=localhost port=5432 dbname=test_db user=test_user password=test_password sslmode=disable
```

**Health Check:** `pg_isready -U test_user -d test_db`
**Data Persistence:** `~/Docker/icitadel-dev-postgres`
**Memory:** ~100MB

### 2. icitadel-dev-mysql (MySQL 8.4)

**Image:** `mysql:8.4`
**Port:** 3306 (standard)
**Credentials:**
- Root Password: `root_password`
- User: `test_user`
- Password: `test_password`
- Database: `test_db`

**Connection String Pattern:**
```
mysql://test_user:test_password@localhost:3306/test_db
```

**Health Check:** `mysqladmin ping -h localhost -u test_user -ptest_password`
**Data Persistence:** `~/Docker/icitadel-dev-mysql`
**Memory:** ~400MB

### 3. icitadel-dev-mongodb (MongoDB 7)

**Image:** `mongo:7`
**Port:** 27017 (standard)
**Credentials:**
- User: `test_user`
- Password: `test_password`
- Database: `test_db` (auth database: admin)

**Connection String Pattern:**
```
mongodb://test_user:test_password@localhost:27017/test_db?authSource=admin
```

**Health Check:** `mongosh --eval "db.adminCommand('ping')"`
**Data Persistence:** `~/Docker/icitadel-dev-mongodb`
**Memory:** ~300MB

### 4. icitadel-dev-mssql (SQL Server 2022)

**Image:** `mcr.microsoft.com/mssql/server:2022-latest`
**Port:** 1433 (standard)
**Credentials:**
- User: `sa`
- Password: `TestPassword123!`
- Database: `master` (default)

**Connection String Pattern:**
```
Server=localhost,1433;User Id=sa;Password=TestPassword123!;TrustServerCertificate=true
```

**Health Check:** `/opt/mssql-tools/bin/sqlcmd -S localhost -U sa -P TestPassword123! -Q 'SELECT 1'`
**Data Persistence:** `~/Docker/icitadel-dev-mssql`
**Memory:** ~2GB (minimum)
**License:** Developer edition (free for dev/test)

### 5. icitadel-dev-oracle (Oracle XE 21)

**Image:** `gvenzl/oracle-xe:21-slim`
**Port:** 1521 (standard)
**Credentials:**
- Admin Password: `test_password`
- User: `test_user`
- Password: `test_password`
- SID: `XE`

**Connection String Pattern:**
```
test_user/test_password@localhost:1521/XE
```

**Health Check:** `healthcheck.sh` (built-in)
**Data Persistence:** `~/Docker/icitadel-dev-oracle`
**Memory:** ~2GB (minimum, uses 1GB shared memory)
**Startup Time:** 1-2 minutes on first initialization

## DatabaseType Mapping

Maps container names to rusty-data's `DatabaseType` enum:

| Container Name | DatabaseType Enum | Default Port |
|----------------|-------------------|--------------|
| `icitadel-dev-postgres` | `DatabaseType::Postgres` | 5432 |
| `icitadel-dev-mysql` | `DatabaseType::MySQL` | 3306 |
| `icitadel-dev-mongodb` | `DatabaseType::MongoDB` | 27017 |
| `icitadel-dev-mssql` | `DatabaseType::SQLServer` | 1433 |
| `icitadel-dev-oracle` | `DatabaseType::Oracle` | 1521 |

## Container Lifecycle Commands

### Using podman-compose (Preferred)

**Start all containers:**
```bash
cd rusty-data
podman-compose up -d
```

**Start specific container:**
```bash
podman-compose up -d icitadel-dev-postgres
```

**Stop all containers:**
```bash
podman-compose down
```

**Stop specific container:**
```bash
podman-compose stop icitadel-dev-mysql
```

**Check status:**
```bash
podman-compose ps
```

**View logs:**
```bash
podman-compose logs icitadel-dev-postgres
podman-compose logs -f icitadel-dev-mysql  # Follow logs
```

### Using podman directly

**List icitadel-dev containers:**
```bash
podman ps --filter "name=icitadel-dev" --format "{{.Names}}\t{{.Status}}\t{{.Ports}}"
```

**Check if container is running:**
```bash
podman ps --filter "name=icitadel-dev-postgres" --filter "status=running" --quiet
```

**Start container:**
```bash
podman start icitadel-dev-postgres
```

**Stop container:**
```bash
podman stop icitadel-dev-postgres
```

**Check container health:**
```bash
podman inspect --format='{{.State.Health.Status}}' icitadel-dev-postgres
```

**Wait for container to be healthy:**
```bash
while [ "$(podman inspect --format='{{.State.Health.Status}}' icitadel-dev-postgres)" != "healthy" ]; do
  echo "Waiting for PostgreSQL..."; sleep 2;
done
```

## ConnectionConfig Generation Pattern

### Template for auto-created connections:

```rust
ConnectionConfig {
    id: format!("local-{}", container_name),  // e.g., "local-postgres"
    name: format!("Local {} (Container)", display_name),  // e.g., "Local PostgreSQL (Container)"
    db_type: database_type,  // From mapping table above
    host: Some("localhost".to_string()),
    port: Some(default_port),  // From mapping table
    database: "test_db".to_string(),  // Most use test_db
    username: Some(username),  // From container credentials
    use_ssl: false,  // Local containers don't use SSL
    parameters: HashMap::new(),
}
```

### Special Cases:

**SQL Server (sa user, no default database):**
```rust
ConnectionConfig {
    id: "local-mssql".to_string(),
    name: "Local SQL Server (Container)".to_string(),
    db_type: DatabaseType::SQLServer,
    host: Some("localhost".to_string()),
    port: Some(1433),
    database: "master".to_string(),  // Use master database
    username: Some("sa".to_string()),  // sa user, not test_user
    use_ssl: false,
    parameters: HashMap::new(),
}
```

**Oracle (uses SID instead of database name):**
```rust
ConnectionConfig {
    id: "local-oracle".to_string(),
    name: "Local Oracle (Container)".to_string(),
    db_type: DatabaseType::Oracle,
    host: Some("localhost".to_string()),
    port: Some(1521),
    database: "XE".to_string(),  // SID for Oracle XE
    username: Some("test_user".to_string()),
    use_ssl: false,
    parameters: HashMap::new(),
}
```

## Error Scenarios to Handle

### Container Not Found
- **Cause:** Container doesn't exist (never created from compose file)
- **Detection:** `podman ps -a --filter "name=icitadel-dev-postgres"` returns empty
- **UI Message:** "Container {name} not found. Please run 'podman-compose up -d' in rusty-data directory."

### Podman Not Available
- **Cause:** podman command not found or not in PATH
- **Detection:** `podman --version` returns error
- **UI Message:** "Podman is not installed or not in PATH. Please install podman to use local containers."

### Container Unhealthy
- **Cause:** Health check failing (database not responding)
- **Detection:** `podman inspect --format='{{.State.Health.Status}}'` returns "unhealthy"
- **UI Message:** "Container {name} is unhealthy. Check logs: podman logs {name}"

### Container Stopped
- **Cause:** Container exists but is not running
- **Detection:** `podman ps` without container, but `podman ps -a` shows it
- **UI Message:** "Container {name} is stopped. Click to start."

### Port Already in Use
- **Cause:** Another service using the standard port
- **Detection:** Container fails to start with port binding error
- **UI Message:** "Port {port} is already in use. Stop the conflicting service or modify port mapping in compose file."

### Compose File Not Found
- **Cause:** rusty-data/podman-compose.yml doesn't exist
- **Detection:** File check fails
- **UI Message:** "podman-compose.yml not found in rusty-data directory."

### Insufficient Resources (SQL Server, Oracle)
- **Cause:** System doesn't have enough memory for heavy databases
- **Detection:** Container starts but immediately exits (OOM killer)
- **UI Message:** "Container {name} requires ~{mem}GB memory. Your system may not have enough resources."

### Permission Denied
- **Cause:** Rootless podman permission issues with bind mounts
- **Detection:** Container logs show permission errors
- **UI Message:** "Permission denied accessing ~/Docker/{name}. Try: podman unshare chown -R 999:999 ~/Docker/{name}"

## Implementation Notes

### Working Directory
All `podman-compose` commands must be executed from the `rusty-data/` directory:
```rust
let working_dir = Path::new("rusty-data");
Command::new("podman-compose")
    .current_dir(working_dir)
    .args(&["up", "-d", "icitadel-dev-postgres"])
    .output()?;
```

### Password Security
Passwords are stored in plain text in compose file (acceptable for local dev containers). DO NOT use these for production or store them in ConnectionConfig's encrypted storage.

### Container State Synchronization
When app starts:
1. List all running `icitadel-dev-*` containers
2. Create ConnectionConfig for each running container
3. Mark as "local" group
4. Remove any stale local connections for stopped containers

### Restart Policy
All containers use `restart: unless-stopped` - they will auto-start on system boot unless explicitly stopped. UI should respect this and not force-start containers the user explicitly stopped.

## Summary

**Total Containers:** 5
**Light Containers (< 500MB):** PostgreSQL, MySQL, MongoDB
**Heavy Containers (2GB+):** SQL Server, Oracle

**Recommended Defaults for UI:**
- Enable by default: PostgreSQL, MySQL, MongoDB
- Disable by default: SQL Server, Oracle (let user opt-in)

**Container Discovery Method:**
```bash
podman ps --filter "name=icitadel-dev" --format "{{.Names}}\t{{.Status}}"
```

This provides both container name and running status in one call.
