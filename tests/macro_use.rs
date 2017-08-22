//! Ensures individual macros can be used - see #54.

#[macro_use(info)]
extern crate log;

#[test]
fn can_import_and_use_just_info() {
    info!("doesn't matter");
}
