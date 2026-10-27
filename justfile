default: check

run *args:
    cargo run --bin tag-track -- {{args}}

run-ai *args:
    AGENT=1 cargo run --bin tag-track -- {{args}}

test:
    cargo test --all-targets

lint:
    cargo fmt --package tag-track-cli -- --check
    cargo clippy --all-targets -- -D warnings

coverage:
    cargo llvm-cov --all-targets --fail-under-lines 90

check: lint test coverage

build:
    cargo build --release --bin tag-track
    mkdir -p bin
    cp target/release/tag-track bin/tag-track
