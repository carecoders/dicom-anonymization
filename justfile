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

# cargo bench --bench anonymization_bench
bench:
    cargo bench --bench anonymization_bench

[working-directory: 'target/criterion/report']
bench-report:
    open index.html

[working-directory: 'bindings/python']
build-python:
    maturin develop

[working-directory: 'bindings/python']
test-python:
    uv run --no-project --with 'maturin,pydicom,pytest' sh -c "maturin develop && pytest"

[working-directory: 'bindings/python']
build-python-release:
    maturin build

[working-directory: 'bindings/wasm']
build-wasm:
    wasm-pack build --target web --out-dir www/pkg

[working-directory: 'bindings/wasm']
serve-wasm:
    python3 -m http.server --directory www 8080

[working-directory: 'dicom-anonymizer-spin']
build-spin:
    spin build

[working-directory: 'dicom-anonymizer-spin']
test-spin:
    cargo test

[working-directory: 'dicom-anonymizer-spin']
run-spin:
    spin up

[working-directory: 'dicom-anonymizer-spin']
deploy-spin:
    spin deploy
