fn main() {
    println!("cargo:rerun-if-changed=templates/");
    println!("cargo:rustc-env=ASKAMA_TEMPLATE_DIR=templates");
}
