# Oracle Database Support - Setup Guide

## Overview

Oracle database support in rusty-data requires the Oracle Instant Client library to be installed on your system. This is a platform-specific requirement that **only affects users who need to connect to Oracle databases**.

If you don't use Oracle, you can safely ignore this guide.

## Why Oracle Instant Client is Required

Unlike other database adapters (PostgreSQL, MySQL, SQLite, MongoDB, MSSQL) which use pure Rust libraries, Oracle connections require the native Oracle Instant Client library (`libclntsh.dylib` on macOS, `libclntsh.so` on Linux).

This is because:
- Oracle's wire protocol is proprietary and complex
- The official Oracle client library is the only supported way to connect
- The Rust `oracle` crate wraps this native library

## Installation

### macOS (Apple Silicon / ARM64)

1. **Download Oracle Instant Client**
   ```bash
   # Visit Oracle's website and download Instant Client for ARM64
   # https://www.oracle.com/database/technologies/instant-client/downloads.html

   # Choose: Instant Client for macOS (ARM64)
   # Download: Basic Package
   ```

2. **Extract and Install**
   ```bash
   # Create installation directory
   mkdir -p ~/oracle

   # Extract the downloaded ZIP
   unzip instantclient-basic-macos.arm64-23.3.0.0.0.zip -d ~/oracle/

   # Verify installation
   ls ~/oracle/instantclient_23_3/libclntsh.dylib
   ```

3. **Set Environment Variable**

   Add to your `~/.zshrc` or `~/.bashrc`:
   ```bash
   export DYLD_LIBRARY_PATH=$HOME/oracle/instantclient_23_3:$DYLD_LIBRARY_PATH
   ```

   Then reload:
   ```bash
   source ~/.zshrc  # or ~/.bashrc
   ```

### macOS (Intel x86_64)

Same as above, but download the Intel version (x86-64) of Instant Client.

### Linux

```bash
# Download Instant Client for Linux
wget https://download.oracle.com/otn_software/linux/instantclient/...

# Extract
unzip instantclient-basic-linux-*.zip -d ~/oracle/

# Set library path in ~/.bashrc
export LD_LIBRARY_PATH=$HOME/oracle/instantclient_23_3:$LD_LIBRARY_PATH

# Update dynamic linker cache (may require sudo)
sudo ldconfig
```

### Windows

1. Download Instant Client Basic for Windows
2. Extract to `C:\oracle\instantclient_23_3`
3. Add to PATH: `C:\oracle\instantclient_23_3`

## Verification

Test your Oracle Instant Client installation:

```bash
# Run Oracle integration tests
cd rusty-data
cargo test --features oracle --test oracle_integration_tests -- --ignored
```

If you see "Cannot locate a 64-bit Oracle Client library" error, the library path is not set correctly.

## Docker/Podman Container Support

The Oracle Database container runs independently of the Instant Client. You need:

1. **Oracle container** (for the database server)
   ```bash
   podman-compose -f compose.yml up -d oracle
   ```

2. **Oracle Instant Client** (for your local application to connect)
   - Installed on your host machine (see above)
   - Not needed inside containers

## Alternative: Skip Oracle Support

If you don't need Oracle database support:

```bash
# Build without Oracle feature
cargo build --features postgres,mysql,sqlite,mongodb,mssql

# rusty-app will work with all other databases
cargo run
```

## Troubleshooting

### "Cannot locate a 64-bit Oracle Client library"

**Cause**: Library path not set or incorrect

**Solution**:
1. Verify library exists: `ls ~/oracle/instantclient_23_3/libclntsh.dylib`
2. Check environment: `echo $DYLD_LIBRARY_PATH` (macOS) or `echo $LD_LIBRARY_PATH` (Linux)
3. Ensure path includes Oracle directory

### "Wrong architecture" errors

**Cause**: Intel library on ARM Mac (or vice versa)

**Solution**: Download the correct architecture version for your Mac

### Container starts but tests fail

**Cause**: Container is running, but local client library missing

**Solution**: Install Oracle Instant Client on your host machine (see above)

## Production Deployment

For production deployments:

1. **Server deployments**: Install Oracle Instant Client on all application servers
2. **Docker/K8s**: Include Oracle Instant Client in your application container image
3. **CI/CD**: Add Instant Client setup to your pipeline

## License

Oracle Instant Client is provided by Oracle Corporation under the Oracle Technology Network License Agreement. It is free for development and production use, but cannot be redistributed. Each user must download it from Oracle's official website.

## Additional Resources

- [Oracle Instant Client Downloads](https://www.oracle.com/database/technologies/instant-client/downloads.html)
- [Rust oracle crate documentation](https://docs.rs/oracle/)
- [ODPI-C Installation Guide](https://oracle.github.io/odpi/doc/installation.html)
