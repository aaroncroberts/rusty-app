# Integration Testing with Test Databases

This document describes how to run integration tests for rusty-data database adapters using Podman containers.

## Prerequisites

- [Podman](https://podman.io/getting-started/installation) installed
- [podman-compose](https://github.com/containers/podman-compose) installed

```bash
# Install podman-compose (if not already installed)
pip3 install podman-compose
```

## Starting Test Databases

The `compose.yml` file provides test database containers for all supported database systems.

### Start all databases

```bash
podman-compose up -d
```

### Start specific database(s)

```bash
podman-compose up -d postgres mysql
```

### Check container status

```bash
podman-compose ps
```

### View logs

```bash
# All containers
podman-compose logs

# Specific container
podman-compose logs postgres
```

## Database Connection Details

All databases are configured with test credentials:

### PostgreSQL (Port 5432)
- **Host**: localhost
- **Port**: 5432
- **Database**: test_db
- **User**: test_user
- **Password**: test_password

### MySQL (Port 3306)
- **Host**: localhost
- **Port**: 3306
- **Database**: test_db
- **User**: test_user
- **Password**: test_password
- **Root Password**: root_password

### Microsoft SQL Server (Port 1433)
- **Host**: localhost
- **Port**: 1433
- **Database**: master (default)
- **User**: sa
- **Password**: Test_Password123!

### Oracle Database XE (Port 1521)
- **Host**: localhost
- **Port**: 1521
- **Service Name**: XE
- **User**: system
- **Password**: Test_Password123!
- **Enterprise Manager**: http://localhost:5500/em

### MongoDB (Port 27017)
- **Host**: localhost
- **Port**: 27017
- **Database**: test_db
- **User**: test_user
- **Password**: test_password
- **Auth Database**: admin

## Running Integration Tests

### Run all integration tests

```bash
cargo test --features all-databases -- --test-threads=1
```

### Run tests for specific database

```bash
# PostgreSQL
cargo test --features postgres -- --test-threads=1

# MySQL
cargo test --features mysql -- --test-threads=1

# SQLite (no container needed)
cargo test --features sqlite

# MongoDB
cargo test --features mongodb -- --test-threads=1

# SQL Server
cargo test --features mssql -- --test-threads=1

# Oracle
cargo test --features oracle -- --test-threads=1
```

**Note**: The `--test-threads=1` flag ensures tests run sequentially to avoid database connection conflicts.

## Health Checks

All containers include health checks. Wait for all services to be healthy before running tests:

```bash
# Check health status
podman-compose ps

# Wait for all containers to be healthy
while ! podman-compose ps | grep -q "healthy"; do
  echo "Waiting for databases to be healthy..."
  sleep 2
done
```

## Stopping and Cleaning Up

### Stop containers (keep data)

```bash
podman-compose stop
```

### Stop and remove containers (keep volumes)

```bash
podman-compose down
```

### Remove containers and volumes (clean slate)

```bash
podman-compose down -v
```

## Troubleshooting

### Oracle container fails to start

Oracle XE requires significant memory and startup time (60+ seconds). Check:

```bash
# View Oracle logs
podman-compose logs oracle

# Increase shm_size if needed (already set to 1GB in compose.yml)
```

### MSSQL health check fails

MSSQL tools take time to initialize. Wait 30-60 seconds and check:

```bash
podman-compose logs mssql
```

### Port conflicts

If ports are already in use, modify `compose.yml` to use different host ports:

```yaml
ports:
  - "15432:5432"  # Use port 15432 instead of 5432
```

## Writing Integration Tests

Integration tests should:

1. Check if the test database is available (skip if not)
2. Create test data
3. Run adapter operations
4. Clean up test data
5. Close connections

Example pattern:

```rust
#[tokio::test]
#[ignore] // Ignored by default, run with --ignored or --include-ignored
async fn test_postgres_integration() {
    let config = ConnectionConfig {
        // ... connection details from above
    };
    
    let mut adapter = PostgresAdapter::new();
    
    // Test connection (skip if unavailable)
    if !adapter.test_connection(&config, Some("test_password")).await.unwrap() {
        println!("PostgreSQL not available, skipping test");
        return;
    }
    
    // Run tests
    adapter.connect(&config, Some("test_password")).await.unwrap();
    // ... test operations ...
    adapter.disconnect().await.unwrap();
}
```

## CI/CD Integration

For CI pipelines, add a step to start test databases:

```yaml
# Example GitHub Actions
steps:
  - name: Start test databases
    run: podman-compose up -d
  
  - name: Wait for databases
    run: |
      timeout 120s bash -c 'until podman-compose ps | grep healthy; do sleep 2; done'
  
  - name: Run integration tests
    run: cargo test --features all-databases -- --test-threads=1
  
  - name: Cleanup
    run: podman-compose down -v
```
