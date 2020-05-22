//! This build script detects target platforms that lack proper support for
//! atomics and sets `cfg` flags accordingly.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let target = env::var("TARGET").unwrap();

    if !target.starts_with("thumbv6") {
        println!("cargo:rustc-cfg=atomic_cas");
    }

    #[cfg(feature = "kv_unstable")]
    {
        let path = Path::new(&env::var_os("OUT_DIR").unwrap()).join("sorted_type_ids.expr.rs");

        fs::write(path, include!("src/kv/value/internal/sorted_type_ids.expr.rs")).unwrap();
    }

    println!("cargo:rerun-if-changed=build.rs");
}
