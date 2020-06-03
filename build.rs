//! This build script detects target platforms that lack proper support for
//! atomics and sets `cfg` flags accordingly.

use std::env;

#[cfg(feature = "kv_unstable")]
#[path = "src/kv/value/internal/cast/primitive.rs"]
mod primitive;

fn main() {
    let target = env::var("TARGET").unwrap();

    if !target.starts_with("thumbv6") {
        println!("cargo:rustc-cfg=atomic_cas");
    }

    #[cfg(feature = "kv_unstable")]
    primitive::generate();

    println!("cargo:rustc-cfg=src_build");

    println!("cargo:rerun-if-changed=build.rs");
}
