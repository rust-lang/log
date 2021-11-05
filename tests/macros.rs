#[cfg(not(lib_build))]
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
fn with_named_args() {
    let cats = "cats";

    info!("hello {cats}", cats = cats);
    info!("hello {cats}", cats = cats,);
    info!("hello {cats}", cats = cats,);
}

#[test]
#[cfg(feature = "kv_unstable")]
fn kv() {
    info!(cat_1 = "chashu", cat_2 = "nori"; "hello {}", "cats");
    info!(target: "my_target", cat_1 = "chashu", cat_2 = "nori"; "hello {}", "cats");
    log!(target: "my_target", log::Level::Warn, cat_1 = "chashu", cat_2 = "nori"; "hello {}", "cats");
}

#[test]
#[cfg(feature = "kv_unstable")]
fn kv_expr_context() {
    match "chashu" {
        cat_1 => {
            info!(target: "target", cat_1 = cat_1, cat_2 = "nori"; "hello {}", "cats")
        }
    };
}
