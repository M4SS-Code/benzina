use std::fmt;

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Unexpected, Visitor},
};

use crate::{U15, U31, U63};

macro_rules! impl_serde_numbers_visit {
    ($type:ident = [$($visit_fn:ident => $kind:ident($inner:ident) => $new_fn:ident),*]) => {
        $(
            fn $visit_fn<E: de::Error>(self, v: $inner) -> Result<Self::Value, E> {
                v.try_into()
                    .map_err(|_| de::Error::invalid_value(Unexpected::$kind(v.into()), &self))
                    .and_then(|cv| {
                        $type::$new_fn(cv)
                            .ok_or_else(|| de::Error::invalid_value(Unexpected::$kind(v.into()), &self))
                    })
            }
        )*
    }
}

macro_rules! impl_serde_numbers {
    ($($type:ident => $inner:ident, $deserialize_fn:ident),*) => {
        $(
            impl Serialize for $type {
                fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    self.get().serialize(serializer)
                }
            }

            impl<'de> Deserialize<'de> for $type {
                fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    struct NumberVisitor;

                    impl Visitor<'_> for NumberVisitor {
                        type Value = $type;

                        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                            f.write_str(stringify!($type))
                        }

                        impl_serde_numbers_visit! {
                            $type = [
                                visit_u8 => Unsigned(u8) => new,
                                visit_u16 => Unsigned(u16) => new,
                                visit_u32 => Unsigned(u32) => new,
                                visit_u64 => Unsigned(u64) => new,
                                visit_i8 => Signed(i8) => new_signed,
                                visit_i16 => Signed(i16) => new_signed,
                                visit_i32 => Signed(i32) => new_signed,
                                visit_i64 => Signed(i64) => new_signed
                            ]
                        }
                    }

                    deserializer.$deserialize_fn(NumberVisitor)
                }
            }
        )*
    }
}

impl_serde_numbers! {
    U15 => u16, deserialize_u16,
    U31 => u32, deserialize_u32,
    U63 => u64, deserialize_u64
}
