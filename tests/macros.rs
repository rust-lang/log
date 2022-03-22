#[cfg(not(lib_build))]
#[macro_use]
extern crate log;

macro_rules! all_log_macros {
    ($($arg:tt)*) => ({
        trace!($($arg)*);
        debug!($($arg)*);
        info!($($arg)*);
        warn!($($arg)*);
        error!($($arg)*);
    });
}

#[test]
fn no_args() {
    for lvl in log::Level::iter() {
        log!(lvl, "hello");
        log!(lvl, "hello",);

        log!(target: "my_target", lvl, "hello");
        log!(target: "my_target", lvl, "hello",);

        log!(lvl, "hello");
        log!(lvl, "hello",);
    }

    all_log_macros!("hello");
    all_log_macros!("hello",);

    all_log_macros!(target: "my_target", "hello");
    all_log_macros!(target: "my_target", "hello",);
}

#[test]
fn anonymous_args() {
    for lvl in log::Level::iter() {
        log!(lvl, "hello {}", "world");
        log!(lvl, "hello {}", "world",);

        log!(target: "my_target", lvl, "hello {}", "world");
        log!(target: "my_target", lvl, "hello {}", "world",);

        log!(lvl, "hello {}", "world");
        log!(lvl, "hello {}", "world",);
    }

    all_log_macros!("hello {}", "world");
    all_log_macros!("hello {}", "world",);

    all_log_macros!(target: "my_target", "hello {}", "world");
    all_log_macros!(target: "my_target", "hello {}", "world",);
}

#[test]
fn named_args() {
    for lvl in log::Level::iter() {
        log!(lvl, "hello {world}", world = "world");
        log!(lvl, "hello {world}", world = "world",);

        log!(target: "my_target", lvl, "hello {world}", world = "world");
        log!(target: "my_target", lvl, "hello {world}", world = "world",);

        log!(lvl, "hello {world}", world = "world");
        log!(lvl, "hello {world}", world = "world",);
    }

    all_log_macros!("hello {world}", world = "world");
    all_log_macros!("hello {world}", world = "world",);

    all_log_macros!(target: "my_target", "hello {world}", world = "world");
    all_log_macros!(target: "my_target", "hello {world}", world = "world",);
}

#[test]
fn enabled() {
    for lvl in log::Level::iter() {
        let _enabled = if log_enabled!(target: "my_target", lvl) {
            true
        } else {
            false
        };
    }
}

#[test]
fn expr() {
    for lvl in log::Level::iter() {
        let _ = log!(lvl, "hello");
    }
}

#[test]
#[cfg(feature = "kv_unstable")]
fn kv_no_args() {
    for lvl in log::Level::iter() {
        log!(target: "my_target", lvl, cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello");

        log!(lvl, cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello");
    }

    all_log_macros!(target: "my_target", cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello");
    all_log_macros!(target = "my_target", cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello");
    all_log_macros!(cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello");
}

#[test]
#[cfg(feature = "kv_unstable")]
fn kv_expr_args() {
    for lvl in log::Level::iter() {
        log!(target: "my_target", lvl, cat_math = { let mut x = 0; x += 1; x + 1 }; "hello");

        log!(lvl, target = "my_target", cat_math = { let mut x = 0; x += 1; x + 1 }; "hello");
        log!(lvl, cat_math = { let mut x = 0; x += 1; x + 1 }; "hello");
    }

    all_log_macros!(target: "my_target", cat_math = { let mut x = 0; x += 1; x + 1 }; "hello");
    all_log_macros!(target = "my_target", cat_math = { let mut x = 0; x += 1; x + 1 }; "hello");
    all_log_macros!(cat_math = { let mut x = 0; x += 1; x + 1 }; "hello");
}

#[test]
#[cfg(feature = "kv_unstable")]
fn kv_anonymous_args() {
    for lvl in log::Level::iter() {
        log!(target: "my_target", lvl, cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {}", "world");
        log!(lvl, target = "my_target", cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {}", "world");

        log!(lvl, cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {}", "world");
    }

    all_log_macros!(target: "my_target", cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {}", "world");
    all_log_macros!(target = "my_target", cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {}", "world");
    all_log_macros!(cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {}", "world");
}

#[test]
#[cfg(feature = "kv_unstable")]
fn kv_named_args() {
    for lvl in log::Level::iter() {
        log!(target: "my_target", lvl, cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {world}", world = "world");
        log!(lvl, target = "my_target", cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {world}", world = "world");

        log!(lvl, cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {world}", world = "world");
    }

    all_log_macros!(target: "my_target", cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {world}", world = "world");
    all_log_macros!(target = "my_target", cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {world}", world = "world");
    all_log_macros!(cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {world}", world = "world");
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

#[test]
fn implicit_named_args() {
    #[rustversion::since(1.58)]
    fn _check() {
        let world = "world";

        for lvl in log::Level::iter() {
            log!(lvl, "hello {world}");
            log!(lvl, "hello {world}",);

            log!(target: "my_target", lvl, "hello {world}");
            log!(target: "my_target", lvl, "hello {world}",);

            log!(lvl, "hello {world}");
            log!(lvl, "hello {world}",);
        }

        all_log_macros!("hello {world}");
        all_log_macros!("hello {world}",);

        all_log_macros!(target: "my_target", "hello {world}");
        all_log_macros!(target: "my_target", "hello {world}",);

        all_log_macros!(target = "my_target"; "hello {world}");
        all_log_macros!(target = "my_target"; "hello {world}",);
    }
}

#[test]
#[cfg(feature = "kv_unstable")]
fn kv_implicit_named_args() {
    #[rustversion::since(1.58)]
    fn _check() {
        let world = "world";

        for lvl in log::Level::iter() {
            log!(target: "my_target", lvl, cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {world}");

            log!(lvl, cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {world}");
        }

        all_log_macros!(target: "my_target", cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {world}");
        all_log_macros!(target = "my_target", cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {world}");
        all_log_macros!(cat_1 = "chashu", cat_2 = "nori", cat_count = 2; "hello {world}");
    }
}

#[test]
#[cfg(feature = "kv_unstable")]
fn kv_string_keys() {
    for lvl in log::Level::iter() {
        log!(target: "my_target", lvl, "also dogs" = "Fílos", "key/that-can't/be/an/ident" = "hi"; "hello {world}", world = "world");
    }

    all_log_macros!(target: "my_target", "also dogs" = "Fílos", "key/that-can't/be/an/ident" = "hi"; "hello {world}", world = "world");
}

/// Some and None (from Option) are used in the macros.
#[derive(Debug)]
enum Type {
    Some,
    None,
}

#[test]
fn regression_issue_494() {
    use self::Type::*;
    all_log_macros!("some message: {:?}, {:?}", None, Some);
}
