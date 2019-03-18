use std::path::PathBuf;

fn main() {
    let lib =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("expected CARGO_MANIFEST_DIR"))
            .join("lib");

    if !lib.is_dir() {
        panic!("missing directory: {}", lib.display());
    }

    println!("cargo:rustc-link-search=native={}", lib.display());
}
