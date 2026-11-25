# NOTE: Rustfmt, Clippy, and other development tooling uses the nightly channel.
tool-channel := "nightly"

# Format the project.
format:
    cargo +{{ tool-channel }} fmt

# Lint all Rust workspace targets with Clippy.
lint:
    cargo +{{ tool-channel }} clippy --workspace --all-targets --locked -- -D warnings

# Lint the project, making use of Dylint and the custom lintset.
dylint:
    cargo +{{ tool-channel }} dylint --all -- --workspace --all-targets --locked

# Run Cargo's default check.
check:
    cargo check

# Run the Rust workspace tests.
test:
    cargo test --workspace --locked
