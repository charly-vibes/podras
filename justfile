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

# Full local gate, in the same order CI runs it.
ci: fmt lint test