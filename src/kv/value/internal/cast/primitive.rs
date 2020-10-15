/*
This module generates code to try efficiently convert some arbitrary `T: 'static` into
a `Primitive`. It's used by both the `build.rs` script and by the source itself. There
are currently two implementations here:

- When the compiler version is less than `1.46.0` we check type ids at runtime. This
means generating a pre-sorted list of type ids at compile time using the `build.rs`
and matching them at runtime.
- When the compiler version is at least `1.46.0` we use const evaluation to check type ids
at compile time. There's no generated code from `build.rs` involved.

In the future when `min_specialization` is stabilized we could use it instead and avoid needing
the `'static` bound altogether.
*/

// Use a build-time generated set of type ids to match a type with a conversion fn
#[cfg(srcbuild)]
pub(super) fn from_any<'v>(
    value: &'v (dyn std::any::Any + 'static),
) -> Option<crate::kv::value::internal::Primitive<'v>> {
    // The set of type ids that map to primitives are generated at build-time
    // by the contents of `sorted_type_ids.expr`. These type ids are pre-sorted
    // so that they can be searched efficiently. See the `sorted_type_ids.expr.rs`
    // file for the set of types that appear in this list
    let type_ids = include!(concat!(env!("OUT_DIR"), "/into_primitive.rs"));

    if let Ok(i) = type_ids.binary_search_by_key(&value.type_id(), |&(k, _)| k) {
        Some((type_ids[i].1)(value))
    } else {
        None
    }
}

// When the `src_build` config is not set then we're in the build script
#[cfg(not(srcbuild))]
#[allow(dead_code)]
pub fn generate() {
    use std::path::Path;
    use std::{env, fs};

    macro_rules! type_ids {
        ($($ty:ty,)*) => {
            [
                $(
                    (
                        std::any::TypeId::of::<$ty>(),
                        stringify!(
                            (
                                std::any::TypeId::of::<$ty>(),
                                (|value| unsafe {
                                    debug_assert_eq!(value.type_id(), std::any::TypeId::of::<$ty>());

                                    // SAFETY: We verify the value is $ty before casting
                                    let value = *(value as *const dyn std::any::Any as *const $ty);
                                    crate::kv::value::internal::Primitive::from(value)
                                }) as for<'a> fn(&'a (dyn std::any::Any + 'static)) -> crate::kv::value::internal::Primitive<'a>
                            )
                        )
                    ),
                )*
                $(
                    (
                        std::any::TypeId::of::<Option<$ty>>(),
                        stringify!(
                            (
                                std::any::TypeId::of::<Option<$ty>>(),
                                (|value| unsafe {
                                    debug_assert_eq!(value.type_id(), std::any::TypeId::of::<Option<$ty>>());

                                    // SAFETY: We verify the value is Option<$ty> before casting
                                    let value = *(value as *const dyn std::any::Any as *const Option<$ty>);
                                    if let Some(value) = value {
                                        crate::kv::value::internal::Primitive::from(value)
                                    } else {
                                        crate::kv::value::internal::Primitive::None
                                    }
                                }) as for<'a> fn(&'a (dyn std::any::Any + 'static)) -> crate::kv::value::internal::Primitive<'a>
                            )
                        )
                    ),
                )*
            ]
        };
    }

    // NOTE: The types here *must* match the ones used above when `const_type_id` is available
    let mut type_ids = type_ids![
        usize,
        u8,
        u16,
        u32,
        u64,
        isize,
        i8,
        i16,
        i32,
        i64,
        f32,
        f64,
        char,
        bool,
        &'static str,
    ];

    type_ids.sort_by_key(|&(k, _)| k);

    let mut ordered_type_ids_expr = String::new();

    ordered_type_ids_expr.push('[');

    for (_, v) in &type_ids {
        ordered_type_ids_expr.push_str(v);
        ordered_type_ids_expr.push(',');
    }

    ordered_type_ids_expr.push(']');

    let path = Path::new(&env::var_os("OUT_DIR").unwrap()).join("into_primitive.rs");
    fs::write(path, ordered_type_ids_expr).unwrap();
}
