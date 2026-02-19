# https://just.systems

default:
    @echo 'Quest Log - A gamified TODO-list for children'
    @just --list

test *ARGS:
    cargo test --lib {{ ARGS }}

verify *ARGS:
    cargo test --test "*integration" {{ ARGS }}

fmt *ARGS='':
    cargo fmt {{ ARGS }}
    deno fmt {{ ARGS }} static/

lint:
    just fmt --check
    cargo clippy
    deno lint static/js/

check: lint test verify
    cargo check

dev *ARGS='':
    cargo run -- {{ ARGS }}

# Requires: cargo install cargo-watch

# Watches src/, templates/, static/ for changes with 500ms debounce
watch:
    cargo watch --delay 1 --exec run --notify --clear

clean:
    cargo clean
