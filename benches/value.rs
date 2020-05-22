#![cfg(feature = "kv_unstable")]
#![feature(test)]

extern crate test;
extern crate log;

use log::kv::Value;

#[bench]
fn u8_to_value(b: &mut test::Bencher) {
    b.iter(|| {
        Value::from(1u8)
    })
}

#[bench]
fn u8_to_value_debug(b: &mut test::Bencher) {
    b.iter(|| {
        Value::from_debug(&1u8)
    })
}

#[bench]
fn str_to_value_debug(b: &mut test::Bencher) {
    b.iter(|| {
        Value::from_debug(&"a string")
    })
}