#[macro_use]
extern crate log;

#[test]
fn kv_info() {
    info!("hello");
    info!("hello",);
    info!("hello {}", "cats");
    info!("hello {}", "cats",);
    info!("hello {}", "cats",);
    info!("hello {}", "cats", {
        cat_1: "chashu",
        cat_2: "nori",
    });
    info!("hello {value}", value = "cats", {
        cat_1: "chashu",
        cat_2: "nori",
    });
}
