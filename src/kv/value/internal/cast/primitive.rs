// Use consts to match a type with a conversion fn
// Pros: fast, will work on stable soon (possibly 1.45.0)
// Cons: requires a `'static` bound
#[cfg(all(src_build, feature = "kv_unstable_const_primitive"))]
pub(super) fn from_any<'v, T: ?Sized + 'static>(value: &'v T) -> Option<crate::kv::value::internal::Primitive<'v>> {
    use std::any::TypeId;

    use crate::kv::value::internal::Primitive;
    
    trait ToPrimitive where Self: 'static {
        const CALL: fn(&Self) -> Option<Primitive> = {
            const U8: TypeId = TypeId::of::<u8>();
            const U16: TypeId = TypeId::of::<u16>();
            const U32: TypeId = TypeId::of::<u32>();
            const U64: TypeId = TypeId::of::<u64>();
    
            const I8: TypeId = TypeId::of::<i8>();
            const I16: TypeId = TypeId::of::<i16>();
            const I32: TypeId = TypeId::of::<i32>();
            const I64: TypeId = TypeId::of::<i64>();
    
            const STR: TypeId = TypeId::of::<&'static str>();
    
            match TypeId::of::<Self>() {
                U8 => |v| Some(Primitive::from(unsafe { *(v as *const Self as *const u8) })),
                U16 => |v| Some(Primitive::from(unsafe { *(v as *const Self as *const u16) })),
                U32 => |v| Some(Primitive::from(unsafe { *(v as *const Self as *const u32) })),
                U64 => |v| Some(Primitive::from(unsafe { *(v as *const Self as *const u64) })),
    
                I8 => |v| Some(Primitive::from(unsafe { *(v as *const Self as *const i8) })),
                I16 => |v| Some(Primitive::from(unsafe { *(v as *const Self as *const i16) })),
                I32 => |v| Some(Primitive::from(unsafe { *(v as *const Self as *const i32) })),
                I64 => |v| Some(Primitive::from(unsafe { *(v as *const Self as *const i64) })),
    
                STR => |v| Some(Primitive::from(unsafe { *(v as *const Self as *const &'static str) })),
    
                _ => |_| None,
            }
        };
    
        fn to_primitive(&self) -> Option<Primitive> {
            (Self::CALL)(self)
        }
    }
    
    impl<T: ?Sized + 'static> ToPrimitive for T { }

    value.to_primitive()
}

#[cfg(all(not(src_build), feature = "kv_unstable_const_primitive"))]
pub fn generate() { }

// Use specialization to match a type with a conversion fn
// Pros: fast, doesn't require `'static` bound
// Cons: might not stabilize for a long time, doesn't work with `&str`
#[cfg(all(src_build, feature = "kv_unstable_spec_primitive"))]
pub(super) fn from_any<'v, T: ?Sized>(value: &'v T) -> Option<crate::kv::value::internal::Primitive<'v>> {
    use std::any::TypeId;

    use crate::kv::value::internal::Primitive;
    
    trait ToPrimitive {
        fn to_primitive(&self) -> Option<Primitive>;
    }

    impl<T: ?Sized> ToPrimitive for T {
        default fn to_primitive(&self) -> Option<Primitive> {
            None
        }
    }

    impl ToPrimitive for u8 {
        #[inline]
        fn to_primitive(&self) -> Option<Primitive> {
            Some(Primitive::from(*self))
        }
    }

    impl ToPrimitive for u16 {
        #[inline]
        fn to_primitive(&self) -> Option<Primitive> {
            Some(Primitive::from(*self))
        }
    }

    impl ToPrimitive for u32 {
        #[inline]
        fn to_primitive(&self) -> Option<Primitive> {
            Some(Primitive::from(*self))
        }
    }

    impl ToPrimitive for u64 {
        #[inline]
        fn to_primitive(&self) -> Option<Primitive> {
            Some(Primitive::from(*self))
        }
    }

    impl ToPrimitive for i8 {
        #[inline]
        fn to_primitive(&self) -> Option<Primitive> {
            Some(Primitive::from(*self))
        }
    }

    impl ToPrimitive for i16 {
        #[inline]
        fn to_primitive(&self) -> Option<Primitive> {
            Some(Primitive::from(*self))
        }
    }

    impl ToPrimitive for i32 {
        #[inline]
        fn to_primitive(&self) -> Option<Primitive> {
            Some(Primitive::from(*self))
        }
    }

    impl ToPrimitive for i64 {
        #[inline]
        fn to_primitive(&self) -> Option<Primitive> {
            Some(Primitive::from(*self))
        }
    }

    impl ToPrimitive for str {
        #[inline]
        fn to_primitive(&self) -> Option<Primitive> {
            Some(Primitive::from(self))
        }
    }

    value.to_primitive()
}

#[cfg(all(not(src_build), feature = "kv_unstable_spec_primitive"))]
pub fn generate() { }

// Use a build-time generated set of type ids to match a type with a conversion fn
// Pros: works on stable
// Cons: not 'free', complicates build script, requires `'static` bound
#[cfg(all(src_build, not(any(feature = "kv_unstable_const_primitive", feature = "kv_unstable_spec_primitive"))))]
pub(in kv::value) fn from_any<'v>(value: &'v (dyn std::any::Any + 'static)) -> Option<crate::kv::value::internal::Primitive<'v>> {
    // The set of type ids that map to primitives are generated at build-time
    // by the contents of `sorted_type_ids.expr`. These type ids are pre-sorted
    // so that they can be searched efficiently. See the `sorted_type_ids.expr.rs`
    // file for the set of types that appear in this list
    const TYPE_IDS: [(std::any::TypeId, for<'a> fn(&'a (dyn std::any::Any + 'static)) -> crate::kv::value::internal::Primitive<'a>); 30] = include!(concat!(env!("OUT_DIR"), "/into_primitive.rs"));

    debug_assert!(TYPE_IDS.is_sorted_by_key(|&(k, _)| k));
    if let Ok(i) = TYPE_IDS.binary_search_by_key(&value.type_id(), |&(k, _)| k) {
        Some((TYPE_IDS[i].1)(value))
    } else {
        None
    }
}

// When the `src_build` config is not set then we're in the build script
// This function will generate an expression fragment used by `into_primitive`
#[cfg(all(not(src_build), not(any(feature = "kv_unstable_const_primitive", feature = "kv_unstable_spec_primitive"))))]
pub fn generate() {
    use std::path::Path;
    use std::{fs, env};

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

    let path = Path::new(&env::var_os("OUT_DIR").unwrap()).join("into_primitive.rs");
    fs::write(path, ordered_type_ids_expr).unwrap();
}
