use std::fmt;

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Unexpected, Visitor},
};

use crate::{U15, U31, U63};

macro_rules! impl_serde_numbers {
    ($($type:ident => $inner:ident, $deserialize_fn:ident, $visit_fn:ident),*) => {
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

                        fn $visit_fn<E: de::Error>(self, v: $inner) -> Result<Self::Value, E> {
                            $type::new(v)
                                .ok_or_else(|| de::Error::invalid_value(Unexpected::Unsigned(v.into()), &self))
                        }
                    }

                    deserializer.$deserialize_fn(NumberVisitor)
                }
            }
        )*
    }
}

impl_serde_numbers! {
    U15 => u16, deserialize_u16, visit_u16,
    U31 => u32, deserialize_u32, visit_u32,
    U63 => u64, deserialize_u64, visit_u64
}
