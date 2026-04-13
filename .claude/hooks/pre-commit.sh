#!/bin/bash
# Pre-commit hook for welfare-store

set -e

echo "Running pre-commit checks..."

# Change to repo root
cd "$(git rev-parse --show-toplevel)"

# Format check
echo "Checking formatting..."
cargo fmt --check

# Type check
echo "Checking types..."
cargo check --all-targets

# Clippy
echo "Running clippy..."
cargo clippy -- -D warnings

# Tests
echo "Running tests..."
cargo test --all-targets

echo "All checks passed!"
