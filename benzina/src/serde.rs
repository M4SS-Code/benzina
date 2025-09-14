use std::fmt;

use serde_core::{
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

#[cfg(test)]
mod tests {
    use serde_test::{Token, assert_de_tokens, assert_ser_tokens};

    use crate::{U15, U31, U63};

    macro_rules! int_ser_tests {
        ($($type:ident, $inner:ident, $token_type:ident, $test_name:ident),*) => {
            $(
                #[test]
                fn $test_name() {
                    const VALUE: $inner = $inner::MAX / 2;
                    let v = $type::new(VALUE).unwrap();
                    assert_ser_tokens(
                        &v,
                        &[
                            Token::$token_type(VALUE),
                        ],
                    );
                }
            )*
        }
    }

    macro_rules! int_de_tests {
        ($($type:ident, $inner:ident, $token_type:ident, $token_type_inner:ident, $test_name:ident),*) => {
            $(
                #[test]
                fn $test_name() {
                    const VALUE: $inner = if ($token_type_inner::MAX as u128) < $inner::MAX as u128 {
                        $token_type_inner::MAX as $inner
                    } else {
                        $inner::MAX
                    } / 2;
                    let v = $type::new(VALUE).unwrap();
                    assert_de_tokens(&v, &[Token::$token_type(VALUE as _)]);
                }
            )*
        }
    }

    int_ser_tests! {
        U15, u16, U16, int_ser_u15,
        U31, u32, U32, int_ser_u31,
        U63, u64, U64, int_ser_u63
    }

    int_de_tests! {
        U15, u16, U8, u8, int_de_u15_from_u8,
        U15, u16, U16, u16, int_de_u15_from_u16,
        U15, u16, U32, u32, int_de_u15_from_u32,
        U15, u16, U64, u64, int_de_u15_from_u64,
        U15, u16, I8, i8, int_de_u15_from_i8,
        U15, u16, I16, i16, int_de_u15_from_i16,
        U15, u16, I32, i32, int_de_u15_from_i32,
        U15, u16, I64, i64, int_de_u15_from_i64,
        U31, u32, U8, u8, int_de_u31_from_u8,
        U31, u32, U16, u16, int_de_u31_from_u16,
        U31, u32, U32, u32, int_de_u31_from_u32,
        U31, u32, U64, u64, int_de_u31_from_u64,
        U31, u32, I8, i8, int_de_u31_from_i8,
        U31, u32, I16, i16, int_de_u31_from_i16,
        U31, u32, I32, i32, int_de_u31_from_i32,
        U31, u32, I64, i64, int_de_u31_from_i64,
        U63, u64, U8, u8, int_de_u63_from_u8,
        U63, u64, U16, u16, int_de_u63_from_u16,
        U63, u64, U32, u32, int_de_u63_from_u32,
        U63, u64, U64, u64, int_de_u63_from_u64,
        U63, u64, I8, i8, int_de_u63_from_i8,
        U63, u64, I16, i16, int_de_u63_from_i16,
        U63, u64, I32, i32, int_de_u63_from_i32,
        U63, u64, I64, i64, int_de_u63_from_i64
    }
}
