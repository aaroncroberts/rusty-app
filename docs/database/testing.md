# Database Testing Environment

Podman Compose configuration for running database containers for testing and development.

## Available Databases

All containers follow the naming pattern: `icitadel-dev-{platform}`

| Database | Container Name | Port | Credentials |
|----------|---------------|------|-------------|
| PostgreSQL 16 | `icitadel-dev-postgres` | 5432 | user: `test_user`, pass: `test_password`, db: `test_db` |
| MySQL 8.4 | `icitadel-dev-mysql` | 3306 | user: `test_user`, pass: `test_password`, db: `test_db` |
| MongoDB 7 | `icitadel-dev-mongodb` | 27017 | user: `test_user`, pass: `test_password`, db: `test_db` |
| SQL Server 2022 | `icitadel-dev-mssql` | 1433 | user: `sa`, pass: `TestPassword123!` |
| Oracle XE 21 | `icitadel-dev-oracle` | 1521 | user: `test_user`, pass: `test_password`, SID: `XE` |

## Quick Start

Navigate to the `rusty-data` directory and use podman-compose:

```bash
cd rusty-data

# Start all databases
podman-compose up -d

# Start specific database
podman-compose up -d icitadel-dev-postgres

# Stop all databases
podman-compose down

# Stop and clean up
podman-compose down -v
```

## Container Management

### Check Status

```bash
podman ps --filter "name=icitadel-dev"
```

### View Logs

```bash
# View logs
podman logs icitadel-dev-postgres

# Follow logs
podman logs -f icitadel-dev-mysql
```

### Execute Commands in Container

```bash
# PostgreSQL
podman exec -it icitadel-dev-postgres psql -U test_user -d test_db

# MySQL
podman exec -it icitadel-dev-mysql mysql -u test_user -ptest_password test_db

# MongoDB
podman exec -it icitadel-dev-mongodb mongosh -u test_user -p test_password --authenticationDatabase admin

# SQL Server
podman exec -it icitadel-dev-mssql /opt/mssql-tools/bin/sqlcmd -S localhost -U sa -P 'TestPassword123!'
```

## Integration with rusty-data

### Connection Configuration Examples

**PostgreSQL:**
```rust
ConnectionConfig {
    id: "test-postgres".to_string(),
    name: "Test PostgreSQL".to_string(),
    db_type: DatabaseType::Postgres,
    host: Some("localhost".to_string()),
    port: Some(5432),
    database: "test_db".to_string(),
    username: Some("test_user".to_string()),
    use_ssl: false,
    parameters: HashMap::new(),
}
```

**MySQL:**
```rust
ConnectionConfig {
    id: "test-mysql".to_string(),
    name: "Test MySQL".to_string(),
    db_type: DatabaseType::MySQL,
    host: Some("localhost".to_string()),
    port: Some(3306),
    database: "test_db".to_string(),
    username: Some("test_user".to_string()),
    use_ssl: false,
    parameters: HashMap::new(),
}
```

**SQLite:**
```rust
ConnectionConfig {
    id: "test-sqlite".to_string(),
    name: "Test SQLite".to_string(),
    db_type: DatabaseType::SQLite,
    host: None,
    port: None,
    database: "/path/to/database.db".to_string(),
    username: None,
    use_ssl: false,
    parameters: HashMap::new(),
}
```

## Running Integration Tests

```bash
# Start databases first
cd rusty-data
podman-compose up -d icitadel-dev-postgres icitadel-dev-mysql

# Wait for health checks to pass
podman-compose ps

# Run tests with specific features
cd ..
cargo test --features postgres
cargo test --features mysql
cargo test --features sqlite

# Run all database tests
cargo test --features all-databases
```

## Health Checks

All containers include health checks that verify the database is ready:

```bash
# Check health status
podman-compose ps

# Wait for specific service to be healthy
podman-compose up -d icitadel-dev-postgres && \
  while [ "$(podman inspect --format='{{.State.Health.Status}}' icitadel-dev-postgres)" != "healthy" ]; do
    echo "Waiting for PostgreSQL..."; sleep 2;
  done
```

## Data Persistence

All databases use bind mounts to persist data in `~/Docker/{container-name}`:

- `~/Docker/icitadel-dev-postgres` - PostgreSQL data
- `~/Docker/icitadel-dev-mysql` - MySQL data
- `~/Docker/icitadel-dev-mongodb` - MongoDB data
- `~/Docker/icitadel-dev-mssql` - SQL Server data
- `~/Docker/icitadel-dev-oracle` - Oracle data

**Benefits:**
- Data persists across container restarts and `podman-compose down`
- Easy to backup by copying the directory
- Easy to inspect database files directly
- Simple cleanup by removing the directory

**To reset a database:**

```bash
# Stop containers
podman-compose down

# Remove the data directory
rm -rf ~/Docker/icitadel-dev-postgres

# Restart (will create fresh database)
podman-compose up -d icitadel-dev-postgres
```

## Resource Requirements

Approximate memory requirements:
- PostgreSQL: ~100MB
- MySQL: ~400MB
- MongoDB: ~300MB
- SQL Server: ~2GB (minimum)
- Oracle XE: ~2GB (minimum, uses 1GB shared memory)

For development, start only the databases you need:

```bash
# Start only PostgreSQL and MySQL
podman-compose up -d icitadel-dev-postgres icitadel-dev-mysql
```

## Troubleshooting

### Port Already in Use

Modify the port mapping in `podman-compose.yml`:

```yaml
ports:
  - "15432:5432"  # Use port 15432 on host instead of 5432
```

### Permission Issues

If using rootless podman, ensure proper permissions:

```bash
podman unshare chown -R 999:999 ~/Docker/icitadel-dev-postgres
```

### SQL Server Licensing

SQL Server uses the Developer edition - free for development/testing, not for production.

### Oracle Database Initialization

Oracle XE takes 1-2 minutes to initialize on first start. Check logs:

```bash
podman logs -f icitadel-dev-oracle
```

## Integration with Existing Containers

Existing containers:
- `icitadel-dev-mongodb` - MongoDB instance
- `icitadel-dev-d1` - Cloudflare D1 (SQLite) database

These can be used alongside the podman-compose setup. Ensure port mappings don't overlap.
