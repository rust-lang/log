#[macro_use]
extern crate log;

#[test]
fn base() {
    info!("hello");
    info!("hello",);
}

#[test]
fn with_args() {
    info!("hello {}", "cats");
    info!("hello {}", "cats",);
    info!("hello {}", "cats",);
}

#[test]
fn named_arg() {
    info!("hello {value}", value = "cats");
}

#[test]
fn kv() {
    info!("hello {}", "cats", {
        cat_1: "chashu",
        cat_2: "nori",
    });
}

#[test]
fn kv_and_named_arg() {
    info!("hello {value}", value = "cats", {
        cat_1: "chashu",
        cat_2: "nori",
    });
}
