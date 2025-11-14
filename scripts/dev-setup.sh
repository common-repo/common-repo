#!/usr/bin/env bash
set -e

echo "Setting up development environment for common-repo..."

if ! command -v cargo &> /dev/null; then
    echo "Error: Rust is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

echo "Checking Rust version..."
rustc --version
cargo --version

if ! command -v cargo-nextest &> /dev/null; then
    echo "Installing cargo-nextest..."
    
    if command -v cargo-binstall &> /dev/null; then
        echo "Using cargo-binstall for faster installation..."
        cargo binstall cargo-nextest --no-confirm
    else
        echo "Installing via cargo install (this may take a few minutes)..."
        cargo install cargo-nextest --locked
    fi
    
    echo "cargo-nextest installed successfully!"
else
    echo "cargo-nextest is already installed ($(cargo nextest --version))"
fi

if ! command -v pre-commit &> /dev/null; then
    echo "Warning: pre-commit is not installed. Install it with: pip install pre-commit"
    echo "Then run: pre-commit install && pre-commit install --hook-type commit-msg"
else
    echo "Installing pre-commit hooks..."
    pre-commit install
    pre-commit install --hook-type commit-msg
    echo "pre-commit hooks installed successfully!"
fi

echo ""
echo "Development environment setup complete!"
echo ""
echo "Quick start commands:"
echo "  cargo nextest run              # Run unit tests"
echo "  cargo nextest run --features integration-tests  # Run all tests including integration"
echo "  cargo fmt                      # Format code"
echo "  cargo clippy                   # Run linter"
echo "  cargo build --release          # Build release binary"
