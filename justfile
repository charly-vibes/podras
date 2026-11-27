# podras developer tasks. CI runs `just ci`, which must stay green.

# Build the binary and library.
build:
    cargo build

# Install the binary to ~/.cargo/bin.
install:
    cargo install --path . --locked

# Run the full test suite.
test:
    cargo test

# Lint every target with warnings denied.
lint:
    cargo clippy --all-targets -- -D warnings

# Verify formatting without rewriting files.
fmt:
    cargo fmt --all -- --check

# Check committed renders against a fresh render; optional YYYY-MM-DD evaluation date.
brief-drift *args:
    scripts/guards/brief-drift.sh {{args}}

# Capture research observations into the kaya ledger: wai artifacts + git
# history, then rebuild and publish the projections. Skips on a dirty tree and
# serializes concurrent runs under .kaya/capture.lock (pattern:
# charly-vibes/kaya justfile:record-capture @ v0.4.1; `kaya check` verifies the
# published tree too, so publish is part of the capture loop).
record-capture:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p .kaya
    exec 9>.kaya/capture.lock
    flock 9
    if [ -n "$(git status --porcelain)" ]; then
        echo "record-capture: SKIP - working tree is dirty; commit or stash first"
        exit 0
    fi
    kaya import wai --project podras
    kaya import git --project podras --exclude-message-prefix "chore(record):"
    kaya build
    kaya publish

# Full local gate, in the same order CI runs it.
ci: fmt lint test