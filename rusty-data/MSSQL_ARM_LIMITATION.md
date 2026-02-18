# Microsoft SQL Server - ARM Architecture Limitation

## Overview

Microsoft SQL Server (including Azure SQL Edge) does not officially support ARM64 architecture. This affects users on:
- **Apple Silicon Macs** (M1, M2, M3, M4 chips)
- **ARM-based Linux servers**
- **Windows on ARM devices**

## The Problem

When attempting to run SQL Server containers on ARM systems:
- Both `mcr.microsoft.com/mssql/server:2022-latest` and `mcr.microsoft.com/azure-sql-edge:latest` crash with segmentation fault (exit code 139)
- Rosetta emulation (macOS) doesn't provide sufficient compatibility
- No native ARM builds are available

## Solutions for ARM Users

### Option 1: Use x86-64 System for MSSQL Testing

If you need to test SQL Server integration:
1. Use a cloud VM (AWS EC2 x86-64, Azure VM, Google Cloud)
2. Use a separate x86-64 machine
3. Use GitHub Actions CI/CD which runs on x86-64

### Option 2: Skip MSSQL Testing on ARM

The rusty-data library supports multiple databases. On ARM systems:

```bash
# Build without MSSQL feature
cargo build --features postgres,mysql,sqlite,mongodb,oracle

# Run tests without MSSQL
cargo test --features postgres,mysql,sqlite,mongodb,oracle
```

Your application will work with all other databases:
- ✅ PostgreSQL (full ARM support)
- ✅ MySQL/MariaDB (full ARM support)
- ✅ SQLite (full ARM support)
- ✅ MongoDB (full ARM support)
- ✅ Oracle (via Instant Client for ARM)
- ❌ MSSQL (x86-64 only)

### Option 3: Use PostgreSQL as Alternative

For local development on ARM, PostgreSQL provides similar features:
- Stored procedures (PL/pgSQL similar to T-SQL)
- Full ACID compliance
- Advanced indexing
- Full-text search
- JSON support

## Production Deployment

In production, SQL Server typically runs on:
- **Windows Server** (x86-64)
- **Linux x86-64** (Ubuntu, RHEL, SUSE)
- **Azure SQL Database** (managed service)

Your application built with MSSQL support will work correctly on these platforms.

## Testing Strategy

### For ARM Developers

1. **Local Development**: Use PostgreSQL, MySQL, or SQLite
2. **CI/CD Testing**: Run MSSQL integration tests in GitHub Actions (x86-64)
3. **Pre-production**: Test on x86-64 staging environment

### For x86-64 Developers

All databases work natively - no limitations.

## Compose File Configuration

The `compose.yml` includes MSSQL configuration that works on x86-64:

```yaml
mssql:
  image: mcr.microsoft.com/mssql/server:2022-latest
  # Will run natively on x86-64
  # Will crash on ARM (no workaround available)
```

**Note**: We don't include `platform: linux/amd64` because:
- On x86-64: Image runs natively
- On ARM: Image crashes even with platform specified

## Microsoft's Official Position

From [Microsoft Documentation](https://learn.microsoft.com/en-us/sql/linux/sql-server-linux-setup):

> SQL Server on Linux supports the following architectures:
> - x64 (64-bit)
> - **ARM64 is not supported**

## Future

Microsoft has not announced plans for ARM support. Track these resources for updates:
- [SQL Server UserVoice](https://feedback.azure.com/d365community/forum/04fe6ee0-3b25-ec11-b6e6-000d3a4f0da0)
- [SQL Server Blog](https://cloudblogs.microsoft.com/sqlserver/)

## Recommendations

**For rusty-app:**

1. ✅ **Mark MSSQL as optional**
   - Use feature flags (already implemented)
   - App works with other 5 databases on all platforms

2. ✅ **Document platform requirements**
   - README should note MSSQL is x86-64 only
   - Provide alternative testing approaches

3. ✅ **CI/CD on x86-64**
   - GitHub Actions runners are x86-64
   - All integration tests can run in CI

4. ✅ **Graceful degradation**
   - App detects available databases
   - Shows appropriate error if MSSQL unavailable on ARM

## Summary

- **MSSQL doesn't work on ARM** - This is a Microsoft limitation, not a rusty-app issue
- **5 out of 6 databases work perfectly on ARM** - PostgreSQL, MySQL, SQLite, MongoDB, Oracle
- **CI/CD testing covers all databases** - Use x86-64 runners for complete testing
- **Production deployments unaffected** - SQL Server runs on x86-64 servers

This limitation affects **local development only** on ARM machines. All features work correctly in production.
