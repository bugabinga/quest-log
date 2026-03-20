//! Adds build options, that Cargo.toml cannot
#[allow(
    clippy::unwrap_used,
    reason = "Build scripts cannot propagate errors, they just fail"
)]
fn main() {
    for entry in glob::glob("static/**/*").unwrap().flatten() {
        if entry.is_file() {
            println!("cargo:rerun-if-changed={}", entry.display());
        }
    }
}
