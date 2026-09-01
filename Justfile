# lolr development tasks

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Build release binary
build:
    cargo build --release

# Run clippy
lint:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Check formatting
fmt-check:
    cargo fmt -- --check

# Run all checks (lint + test)
check: lint test

# Install locally
install:
    cargo install --path .

# Clean build artifacts
clean:
    cargo clean

# Run with sample input
demo:
    echo "Hello, rainbow world!" | cargo run

# Run animated demo
demo-animate:
    echo "Hello, rainbow world!" | cargo run -- -a

# Show help
help:
    cargo run -- --help
