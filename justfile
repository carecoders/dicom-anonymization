# cargo build --release
build:
    cargo build --release

# cargo test
test:
    cargo test

# cargo doc
docs:
    cargo doc --no-deps --lib --package dicom-anonymization

# cargo clippy --all-targets --all-features
clippy:
    cargo clippy --all-targets --all-features

# cargo deny --all-features check
deny:
    cargo deny --all-features check

# cargo +nightly udeps
udeps:
    cargo +nightly udeps

[working-directory: 'bindings/python']
build-python:
    maturin develop

[working-directory: 'bindings/python']
test-python:
    uv run --no-project --with 'maturin,pytest,pydicom' sh -c "maturin develop && pytest"

[working-directory: 'bindings/python']
build-python-release:
    maturin build
