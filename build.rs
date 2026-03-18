//! Adds build options, that Cargo.toml cannot
fn main() {
    // Add static/ to watched dirs in cargo watch
    println!("cargo:rerun-if-changed=static/");
}
