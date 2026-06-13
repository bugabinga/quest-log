# Session: 2026-03-21 WASM Workspace Integration

## Challenges Faced

- **Profile override warning**: Package `[profile.release]` in subcrate was
  ignored because workspace root controls profiles. Solved by moving to
  package-specific override in root `Cargo.toml`.
- **`lto` can't be in package profile**: Cargo only allows `opt-level`, `debug`,
  `debug-assertions`, `overflow-checks`, `panic`, `rpath`, `codegen-units`,
  `strip` in package overrides — not `lto`.
- **Wrong build target**: `cargo build -p` ignored `.cargo/config.toml` and
  tried native target. Solved by passing `--target wasm32-wasip2` explicitly.

## Learnings

- **Workspace member `.cargo/config.toml` IS respected** for default target when
  building that crate directly, but `-p` flag from workspace root requires
  explicit `--target`.
- **WASM size optimization**: `opt-level = "z"` is critical for WASM — release
  build is 117KB vs 3.9MB debug.
