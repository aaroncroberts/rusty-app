#!/usr/bin/env bash
# ci-check.sh — Run the same checks as GitHub Actions CI locally.
# Oracle is excluded (requires native OCI client libs).
set -euo pipefail

CI_FEATURES="postgres,mysql,sqlite,mongodb,mssql"

echo "==> fmt"
cargo fmt --all -- --check

echo "==> check"
cargo check --workspace --features "$CI_FEATURES"

echo "==> clippy"
cargo clippy --workspace --features "$CI_FEATURES" -- -D warnings

echo "==> test"
cargo test --workspace --features "$CI_FEATURES"

echo ""
echo "All checks passed."
