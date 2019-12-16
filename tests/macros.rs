#[macro_use]
extern crate log;

#[test]
fn base() {
    info!("hello");
    info!("hello",);
}

#[test]
fn base_expr_context() {
    let _ = info!("hello");
}

#[test]
fn with_args() {
    info!("hello {}", "cats");
    info!("hello {}", "cats",);
    info!("hello {}", "cats",);
}

#[test]
fn with_args_expr_context() {
    match "cats" {
        cats => info!("hello {}", cats),
    };
}

#[test]
fn kv() {
    info!("hello {}", "cats", {
        cat_1: "chashu",
        cat_2: "nori",
    });
}

#[test]
fn kv_expr_context() {
    match "chashu" {
        cat_1 => info!("hello {}", "cats", {
            cat_1: cat_1,
            cat_2: "nori",
        }),
    };
}
