fn main() {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    println!("cargo:rustc-env=APP_VERSION={}", version);
}
