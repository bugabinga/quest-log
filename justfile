# https://just.systems

default:
    @echo 'Quest Log - A gamified TODO-list for children'
    @just --list

test:
    cargo test --lib

verify:
    cargo test --test "*integration"

lint:
    cargo clippy
    cargo fmt --check

check: lint test
    cargo check

dev:
    cargo run