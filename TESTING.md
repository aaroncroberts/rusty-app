# Testing Guide for rusty-app Database Adapters

This guide explains how to test the database adapters with real databases using Podman.

## Prerequisites

- **Podman** installed and running
- **podman-compose** installed (optional, for convenience)

Install podman-compose:
```bash
pip3 install podman-compose
# OR
brew install podman-compose  # macOS
```

## Quick Start

### Start Test Databases

Using podman-compose (recommended):
```bash
podman-compose up -d
```

Using podman directly:
```bash
# PostgreSQL
podman run -d \
  --name rusty-app-postgres \
  -e POSTGRES_USER=rusty_user \
  -e POSTGRES_PASSWORD=rusty_pass \
  -e POSTGRES_DB=rusty_test \
  -p 5432:5432 \
  -v ./test-data/postgres-init.sql:/docker-entrypoint-initdb.d/init.sql:ro \
  docker.io/library/postgres:16-alpine

# MySQL
podman run -d \
  --name rusty-app-mysql \
  -e MYSQL_ROOT_PASSWORD=root_pass \
  -e MYSQL_DATABASE=rusty_test \
  -e MYSQL_USER=rusty_user \
  -e MYSQL_PASSWORD=rusty_pass \
  -p 3306:3306 \
  -v ./test-data/mysql-init.sql:/docker-entrypoint-initdb.d/init.sql:ro \
  docker.io/library/mysql:8
```

### Verify Databases Are Running

```bash
podman ps
```

You should see containers for both PostgreSQL and MySQL.

### Check Database Health

```bash
# PostgreSQL
podman exec rusty-app-postgres pg_isready -U rusty_user -d rusty_test

# MySQL
podman exec rusty-app-mysql mysqladmin ping -h localhost -u rusty_user -prusty_pass
```

## Connection Details

### PostgreSQL
- **Host**: localhost
- **Port**: 5432
- **Database**: rusty_test
- **Username**: rusty_user
- **Password**: rusty_pass
- **Connection String**: `postgresql://rusty_user:rusty_pass@localhost:5432/rusty_test`

### MySQL
- **Host**: localhost
- **Port**: 3306
- **Database**: rusty_test
- **Username**: rusty_user
- **Password**: rusty_pass
- **Connection String**: `mysql://rusty_user:rusty_pass@localhost:3306/rusty_test`

### SQLite
- **File**: `./test-data/rusty_test.db`
- No container needed (file-based)

## Test Data

Both PostgreSQL and MySQL are initialized with the same test schema:

### Tables
- **users**: id, username, email, created_at, is_active
- **products**: id, name, description, price, stock, created_at
- **orders**: id, user_id, total_amount, status, created_at

### Sample Queries

```sql
-- List all users
SELECT * FROM users;

-- Products with low stock
SELECT * FROM products WHERE stock < 30;

-- Orders by user
SELECT o.id, u.username, o.total_amount, o.status
FROM orders o
JOIN users u ON o.user_id = u.id;
```

## Running Tests

### Integration Tests

```bash
# Test all adapters with real databases
cargo test --package rusty-data --features all-databases -- --test-threads=1

# Test specific adapter
cargo test --package rusty-data --features postgres postgres
cargo test --package rusty-data --features mysql mysql
cargo test --package rusty-data --features sqlite sqlite
```

### Manual Testing

Create a test connection configuration:
```bash
cargo run --package rusty-data --example test_connection
```

## Troubleshooting

### Port Already in Use

If ports 5432 or 3306 are already in use:
```bash
# Check what's using the port
lsof -i :5432
lsof -i :3306

# Stop conflicting services
brew services stop postgresql  # macOS PostgreSQL
brew services stop mysql       # macOS MySQL
```

Or modify `docker-compose.yml` to use different ports:
```yaml
ports:
  - "5433:5432"  # Use port 5433 instead of 5432
```

### Container Won't Start

Check logs:
```bash
podman-compose logs postgres
podman-compose logs mysql
```

### Reset Database

```bash
# Stop and remove containers
podman-compose down -v

# Restart fresh
podman-compose up -d
```

## Cleanup

### Stop Databases

```bash
podman-compose down
```

### Remove All Data (including volumes)

```bash
podman-compose down -v
```

### Remove Individual Containers

```bash
podman stop rusty-app-postgres rusty-app-mysql
podman rm rusty-app-postgres rusty-app-mysql
podman volume rm rusty-app-postgres-data rusty-app-mysql-data
```

## Development Workflow

1. **Start databases**: `podman-compose up -d`
2. **Develop adapters**: Edit code in `rusty-data/src/adapters/`
3. **Run tests**: `cargo test --package rusty-data --features all-databases`
4. **Check logs**: View application logs in `./logs/rusty-app.log`
5. **Stop databases**: `podman-compose down` (when done)

## Notes

- Podman is daemonless and rootless, making it a secure Docker alternative
- podman-compose is compatible with docker-compose.yml syntax
- Test data is automatically loaded on first container start
- Volumes persist data between container restarts
- Use `podman-compose down -v` to reset to fresh state
