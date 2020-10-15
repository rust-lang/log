//! This build script detects target platforms that lack proper support for
//! atomics and sets `cfg` flags accordingly.

use std::env;
use std::process::Command;
use std::str::{self, FromStr};

#[cfg(feature = "kv_unstable")]
#[path = "src/kv/value/internal/cast/primitive.rs"]
mod primitive;

fn main() {
    let target = match rustc_target() {
        Some(target) => target,
        None => return,
    };

    if target_has_atomic_cas(&target) {
        println!("cargo:rustc-cfg=atomic_cas");
    }

    if target_has_atomics(&target) {
        println!("cargo:rustc-cfg=has_atomics");
    }

    // Generate sorted type id lookup
    #[cfg(feature = "kv_unstable")]
    primitive::generate();

    println!("cargo:rustc-cfg=srcbuild");
    println!("cargo:rerun-if-changed=build.rs");
}

fn target_has_atomic_cas(target: &str) -> bool {
    match &target[..] {
        "thumbv6m-none-eabi"
        | "msp430-none-elf"
        | "riscv32i-unknown-none-elf"
        | "riscv32imc-unknown-none-elf" => false,
        _ => true,
    }
}

fn target_has_atomics(target: &str) -> bool {
    match &target[..] {
        "msp430-none-elf" | "riscv32i-unknown-none-elf" | "riscv32imc-unknown-none-elf" => false,
        _ => true,
    }
}

fn rustc_target() -> Option<String> {
    env::var("TARGET").ok()
}
