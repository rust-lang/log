fn main() {
    println!("cargo:rustc-cfg=lib_build");

    println!("cargo:rerun-if-changed=src/build.rs");
}
