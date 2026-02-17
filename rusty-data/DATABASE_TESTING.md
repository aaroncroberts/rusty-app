# Database Testing Environment

This directory contains a Docker Compose configuration for running real database instances for testing and development.

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

### Using Docker Compose

```bash
# Start all databases
docker-compose up -d

# Start specific database
docker-compose up -d icitadel-dev-postgres

# Stop all databases
docker-compose down

# Stop and remove volumes (clean slate)
docker-compose down -v
```

### Using Podman Compose

```bash
# Start all databases
podman-compose up -d

# Start specific database
podman-compose up -d icitadel-dev-postgres

# Stop all databases
podman-compose down

# Stop and remove volumes
podman-compose down -v
```

## Container Management

### Check Status

```bash
# Docker
docker ps --filter "name=icitadel-dev"

# Podman
podman ps --filter "name=icitadel-dev"
```

### View Logs

```bash
# Docker
docker logs icitadel-dev-postgres
docker logs -f icitadel-dev-mysql  # follow logs

# Podman
podman logs icitadel-dev-postgres
podman logs -f icitadel-dev-mysql
```

### Execute Commands in Container

```bash
# PostgreSQL
docker exec -it icitadel-dev-postgres psql -U test_user -d test_db

# MySQL
docker exec -it icitadel-dev-mysql mysql -u test_user -ptest_password test_db

# MongoDB
docker exec -it icitadel-dev-mongodb mongosh -u test_user -p test_password --authenticationDatabase admin

# SQL Server
docker exec -it icitadel-dev-mssql /opt/mssql-tools/bin/sqlcmd -S localhost -U sa -P 'TestPassword123!'
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

**SQLite (existing icitadel-dev-d1):**
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
docker-compose up -d icitadel-dev-postgres icitadel-dev-mysql

# Wait for health checks to pass
docker-compose ps

# Run tests with specific features
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
docker-compose ps

# Wait for specific service to be healthy
docker-compose up -d icitadel-dev-postgres && \
  while [ "$(docker inspect --format='{{.State.Health.Status}}' icitadel-dev-postgres)" != "healthy" ]; do
    echo "Waiting for PostgreSQL..."; sleep 2;
  done
```

## Data Persistence

All databases use bind mounts to persist data in local directories:

- `/Users/aaron/Docker/icitadel-dev-postgres` - PostgreSQL data
- `/Users/aaron/Docker/icitadel-dev-mysql` - MySQL data
- `/Users/aaron/Docker/icitadel-dev-mongodb` - MongoDB data
- `/Users/aaron/Docker/icitadel-dev-mssql` - SQL Server data
- `/Users/aaron/Docker/icitadel-dev-oracle` - Oracle data

Benefits of bind mounts:
- Data persists across container restarts and `docker-compose down`
- Easy to backup by copying the directory
- Easy to inspect database files directly
- Simple cleanup by removing the directory

To reset a database:

```bash
# Stop containers
docker-compose down

# Remove the data directory
rm -rf /Users/aaron/Docker/icitadel-dev-postgres

# Restart (will create fresh database)
docker-compose up -d icitadel-dev-postgres
```

## Resource Requirements

Approximate memory requirements:
- PostgreSQL: ~100MB
- MySQL: ~400MB
- MongoDB: ~300MB
- SQL Server: ~2GB (minimum)
- Oracle XE: ~2GB (minimum, uses 1GB shared memory)

For development, you may want to start only the databases you need:

```bash
# Start only PostgreSQL and MySQL
docker-compose up -d icitadel-dev-postgres icitadel-dev-mysql
```

## Troubleshooting

### Port Already in Use

If a port is already in use, modify the port mapping in `docker-compose.yml`:

```yaml
ports:
  - "15432:5432"  # Use port 15432 on host instead of 5432
```

### Permission Issues with Podman

If using rootless podman, ensure proper permissions:

```bash
podman unshare chown -R 999:999 /path/to/volume
```

### SQL Server Licensing

The SQL Server image uses the Developer edition, which is free for development and testing but not for production use.

### Oracle Database Initialization

Oracle XE takes longer to initialize (1-2 minutes) on first start. Check logs:

```bash
docker logs -f icitadel-dev-oracle
```

## Integration with Existing Containers

As mentioned, you already have:
- `icitadel-dev-mongodb` - MongoDB instance
- `icitadel-dev-d1` - Cloudflare D1 (SQLite) database

These can be used alongside the docker-compose setup. To avoid conflicts, ensure port mappings don't overlap.
