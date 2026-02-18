# Oracle Integration Tests - ARM64 Limitation

## Status

Oracle integration tests are **not compatible with ARM64 (Apple Silicon)** architecture at this time.

## Issue

Oracle Database does not officially support ARM64. When running Oracle containers (either the official `container-registry.oracle.com/database/express` or community `gvenzl/oracle-xe` images) on ARM64 systems via emulation, the PMON background process fails with:

```
ORA-00443: background process "PMON" did not start
```

This is a critical Oracle process that manages process cleanup and cannot run properly under ARM64 emulation.

## Workaround

Oracle integration tests should be run on:
- **x86_64 Linux** systems (native or CI/CD)
- **x86_64 macOS** systems (Intel Macs)
- **Windows x86_64** systems

## Testing on ARM64

For local development on Apple Silicon:
- The other 5 database adapters (PostgreSQL, MySQL, SQLite, MongoDB, MSSQL) work perfectly
- Oracle tests can be run on x86_64 CI servers or cloud environments
- Consider using remote development environments for Oracle-specific work

## CI/CD

Ensure Oracle integration tests run on x86_64 runners in your CI/CD pipeline.

---

**Last Updated:** 2026-02-18
**Tested On:** macOS ARM64 (Apple Silicon)
