/*
An expression fragment for a stringified set of type id to primitive mappings.

This file is used by `build.rs` to generate a pre-sorted list of type ids.
*/

{
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
                                    value.into()
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
                                        value.into()
                                    } else {
                                        Primitive::None
                                    }
                                }) as for<'a> fn(&'a (dyn std::any::Any + 'static)) -> crate::kv::value::internal::Primitive<'a>
                            )
                        )
                    ),
                )*
            ]
        };
    }

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

        &str,
    ];

    type_ids.sort_by_key(|&(k, _)| k);

    let mut ordered_type_ids_expr = String::new();

    ordered_type_ids_expr.push('[');

    for (_, v) in &type_ids {
        ordered_type_ids_expr.push_str(v);
        ordered_type_ids_expr.push(',');
    }

    ordered_type_ids_expr.push(']');

    ordered_type_ids_expr
}
