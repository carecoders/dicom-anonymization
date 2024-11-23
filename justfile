# cargo build --release
build:
    cargo build --release

# cargo test
test:
    cargo test

# cargo doc
docs:
    cargo doc --no-deps

# cargo clippy --all-targets --all-features
clippy:
    cargo clippy --all-targets --all-features

# cargo deny --all-features check
deny:
    cargo deny --all-features check

# cargo +nightly udeps
udeps:
    cargo +nightly udeps
