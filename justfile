# https://just.systems

default:
    @echo 'Quest Log - A gamified TODO-list for children'
    @just --list

test *ARGS:
    cargo test --lib {{ ARGS }}

verify *ARGS:
    cargo test {{ ARGS }}

fmt *ARGS='':
    cargo fmt {{ ARGS }}
    deno fmt {{ ARGS }} static/

lint:
    just fmt --check
    cargo clippy
    deno lint static/js/

check: lint verify
    cargo check

# run app in debug mode
run log_level='debug' subcommand='serve':
    RUST_LOG={{ log_level }} cargo run -- {{ subcommand }}

# Version from git

VERSION := `git describe --tags --always --dirty 2>/dev/null || echo "dev"`
GIT_DIRTY := `git status --porcelain 2>/dev/null`

# Check for dirty git (only for release builds)
_check-clean-for-release:
    #!/usr/bin/env bash
    if [ "{{ GIT_DIRTY }}" != "" ]; then
        echo "Error: Cannot build release with uncommitted changes."
        exit 1
    fi

# Build container image (dev build - allows dirty git)
build-container:
    podman build \
      --build-arg VERSION={{ VERSION }} \
      -t bugabinga/quest-log:local \
      -f Containerfile \
      .

# Build container image (release build - fails if git is dirty)
build-container-release:
    @just _check-clean-for-release
    @just build-container

# Tag image with version
tag-container:
    podman tag bugabinga/quest-log:local bugabinga/quest-log:{{ VERSION }}
    podman tag bugabinga/quest-log:local bugabinga/quest-log:latest
    @echo "Tagged as bugabinga/quest-log:{{ VERSION }} and latest"

# Push to GHCR (dev build)
push-container:
    just build-container
    just tag-container
    podman push bugabinga/quest-log:{{ VERSION }}
    podman push bugabinga/quest-log:latest

# Push to GHCR (release build - fails if git is dirty)
push-container-release:
    just build-container-release
    just tag-container
    podman push bugabinga/quest-log:{{ VERSION }}
    podman push bugabinga/quest-log:latest

# Migrate database in container with specified volume
[arg('volume', long='volume', value='quest-log-data')]
run-migrate volume:
    podman run --rm -v {{ volume }}:/data bugabinga/quest-log:local migrate-only

# Watches src/, templates/, static/ for changes with 500ms debounce
watch:
    cargo watch --delay 1 --exec run --notify --clear

# Remove local development state
clean:
    cargo clean
    rm ./quests.db

# Bundle datastar from jsdelivr CDN
[arg('version')]
bundle-datastar version='1.0.0-RC.8':
    deno bundle \
        --minify \
        https://cdn.jsdelivr.net/gh/starfederation/datastar@{{ version }}/bundles/datastar.js \
        --sourcemap=external \
        -o static/js/datastar.js

# Validate commit message follows conventional commits format
# Usage: just validate-commit-msg <commit-msg-file>
[arg('COMMIT_MSG_FILE')]
validate-commit-msg COMMIT_MSG_FILE:
    #!/usr/bin/env bash
    # Skip for merge commits
    if [ -n "$(git rev-parse -q --verify MERGE_HEAD)" ]; then
        exit 0
    fi
    
    MSG=$(head -n 1 "{{ COMMIT_MSG_FILE }}")
    PATTERN="^(feat|fix|docs|style|refactor|test|chore|perf|revert)(\(.+\))?!?: .+"
    
    if ! echo "$MSG" | grep -qE "$PATTERN"; then
        echo "Invalid commit message format."
        echo ""
        echo "Expected: <type>(<scope>)<!>: <subject>"
        echo "  - Type: feat, fix, docs, style, refactor, test, chore, perf, revert"
        echo "  - Add ! before : for breaking changes"
        echo ""
        echo "Examples:"
        echo "  feat(auth): add login button"
        echo "  fix(ui): resolve padding issue"
        echo "  feat(api)!: remove v1 endpoint"
        echo ""
        echo "Your commit:"
        echo "$MSG"
        exit 1
    fi
    
    # Subject line length check (72 chars)
    SUBJECT=$(echo "$MSG" | sed 's/^[^:]*: //')
    if [ ${#SUBJECT} -gt 72 ]; then
        echo "Subject line exceeds 72 characters (current: ${#SUBJECT})"
        exit 1
    fi
