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

# Run the same checks as CI
check: fmt-check lint test build

# Verify that the current committed tree is ready to publish
release-check: check
    cargo package --locked

# Bump, validate, tag, push, and create a GitHub release.
# Publishing the GitHub release triggers .github/workflows/publish.yml.
release release_version:
    #!/usr/bin/env bash
    set -euo pipefail

    if [[ ! "{{release_version}}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "release version must be plain semver (for example: 0.2.0)" >&2
        exit 2
    fi
    if [[ "$(git branch --show-current)" != "main" ]]; then
        echo "releases must be created from main" >&2
        exit 2
    fi
    if [[ -n "$(git status --porcelain)" ]]; then
        echo "working tree must be clean before releasing" >&2
        exit 2
    fi

    git fetch origin main --tags
    if [[ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]]; then
        echo "local main must match origin/main before releasing" >&2
        exit 2
    fi
    if git rev-parse --verify --quiet "refs/tags/v{{release_version}}" >/dev/null; then
        echo "tag v{{release_version}} already exists" >&2
        exit 2
    fi

    RELEASE_VERSION="{{release_version}}" perl -0pi -e 's/\A(.*?^\[package\].*?^version = ")[^"]+(")/$1$ENV{RELEASE_VERSION}$2/ms' Cargo.toml
    cargo check --quiet
    if ! cargo pkgid | grep -q "@{{release_version}}$"; then
        echo "failed to update package version" >&2
        exit 2
    fi

    git add Cargo.toml Cargo.lock
    git commit -m "chore: bump version to {{release_version}}"
    just release-check

    git tag -a "v{{release_version}}" -m "v{{release_version}}"
    git push --atomic origin main "v{{release_version}}"
    gh release create "v{{release_version}}" --verify-tag --title "v{{release_version}}" --generate-notes

# Show the GitHub release and its publish workflow runs
release-status release_version:
    gh release view "v{{release_version}}"
    gh run list --workflow publish.yml --limit 5

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
