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

# Version from git
VERSION := `git describe --tags --always --dirty 2>/dev/null || echo "dev"`
GIT_DIRTY := `git status --porcelain 2>/dev/null`

# Check for dirty git (only for release builds)
_check-clean-for-release:
    #!/usr/bin/env bash
    if [ "{{GIT_DIRTY}}" != "" ]; then
        echo "Error: Cannot build release with uncommitted changes."
        exit 1
    fi

# Build container image (dev build - allows dirty git)
build-container:
    podman build \
      --build-arg VERSION={{VERSION}} \
      -t bugabinga/quest-log:local \
      -f Containerfile \
      .

# Build container image (release build - fails if git is dirty)
build-container-release:
    @just _check-clean-for-release
    podman build \
      --build-arg VERSION={{VERSION}} \
      -t bugabinga/quest-log:local \
      -f Containerfile \
      .

# Tag image with version
tag-container:
    podman tag bugabinga/quest-log:local bugabinga/quest-log:{{VERSION}}
    podman tag bugabinga/quest-log:local bugabinga/quest-log:latest
    @echo "Tagged as bugabinga/quest-log:{{VERSION}} and latest"

# Push to GHCR (dev build)
push-container:
    just build-container
    just tag-container
    podman push bugabinga/quest-log:{{VERSION}}
    podman push bugabinga/quest-log:latest

# Push to GHCR (release build - fails if git is dirty)
push-container-release:
    just build-container-release
    just tag-container
    podman push bugabinga/quest-log:{{VERSION}}
    podman push bugabinga/quest-log:latest

# Migrate database in container with specified volume
[arg('volume', long='volume', value='quest-log-data')]
run-migrate volume:
    podman run --rm -v {{volume}}:/data bugabinga/quest-log:local migrate-only

# Migrate using local volume (convenience alias)
migrate-container:
    just run-migrate --volume quest-log-data

db-migrate:
    @echo 'Running embedded migrations (migrate-only)'
    mkdir -p ./data
    QUEST_LOG_DATA_DIR=$(pwd)/data cargo run -- migrate-only

# Requires: cargo install cargo-watch

# Watches src/, templates/, static/ for changes with 500ms debounce
watch:
    cargo watch --delay 1 --exec run --notify --clear

clean:
    cargo clean
