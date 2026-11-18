# podras developer tasks. CI runs `just ci`, which must stay green.

# Build the binary and library.
build:
    cargo build

# Run the full test suite.
test:
    cargo test

# Lint every target with warnings denied.
lint:
    cargo clippy --all-targets -- -D warnings

# Verify formatting without rewriting files.
fmt:
    cargo fmt --all -- --check

# Full local gate, in the same order CI runs it.
ci: fmt lint test