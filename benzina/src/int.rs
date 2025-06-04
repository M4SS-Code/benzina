use std::{
    error::Error,
    fmt::{self, Display},
    ptr,
    str::FromStr,
};

use diesel::{
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    pg::{Pg, PgValue},
    serialize::{Output, ToSql},
    sql_types::{BigInt, Integer, SmallInt},
};

use crate::error::{ParseIntError, TryFromIntError};

macro_rules! impl_numbers {
    ($($type:ident => $inner:ident, $inner_signed:ident, $sql_type:ident),*) => {
        $(
            #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[derive(FromSqlRow, AsExpression)]
            #[diesel(sql_type = $sql_type)]
            pub struct $type($inner);

            impl $type {
                pub const BITS: u32 = $inner::BITS - 1;
                pub const MIN: Self = Self(0);
                pub const MAX: Self = Self($inner::MAX >> 1);

                #[must_use]
                pub const fn new(n: $inner) -> Option<Self> {
                    if n <= Self::MAX.get() {
                        Some(Self(n))
                    } else {
                        None
                    }
                }

                #[expect(clippy::cast_sign_loss, reason = "we assert that `n` is positive")]
                #[must_use]
                pub const fn new_signed(n: $inner_signed) -> Option<Self> {
                    if n >= Self::MIN.get_signed() && n <= Self::MAX.get_signed() {
                        Some(Self(n as $inner))
                    } else {
                        None
                    }
                }

                #[must_use]
                pub const fn get(self) -> $inner {
                    self.0
                }

                #[expect(clippy::cast_possible_wrap, reason = "the number is in the positive range of the signed output")]
                #[must_use]
                pub const fn get_signed(self) -> $inner_signed {
                    self.get() as $inner_signed
                }

                #[must_use]
                const fn get_ref(&self) -> &$inner {
                    &self.0
                }

                #[must_use]
                const fn get_signed_ref(&self) -> &$inner_signed {
                    // SAFETY: `&u16`/`&u32`/`&u64` can be interpreted as `&i16`/`&i32`/`&i64` respectively
                    unsafe { &*ptr::from_ref(self.get_ref()).cast::<$inner_signed>() }
                }

                #[must_use]
                pub const fn checked_add(self, rhs: Self) -> Option<Self> {
                    let Some(res) = self.get().checked_add(rhs.get()) else {
                        return None;
                    };

                    if res <= Self::MAX.get() {
                        Some(Self(res))
                    } else {
                        None
                    }
                }

                #[must_use]
                pub const fn saturating_add(self, rhs: Self) -> Self {
                    match self.checked_add(rhs)  {
                        Some(res) => res,
                        None => Self::MAX,
                    }
                }

                #[must_use]
                pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
                    match self.get().checked_sub(rhs.get()) {
                        Some(res) => Some(Self(res)),
                        None => None,
                    }
                }

                #[must_use]
                pub const fn saturating_sub(self, rhs: Self) -> Self {
                    match self.checked_sub(rhs)  {
                        Some(res) => res,
                        None => Self::MIN,
                    }
                }

                #[must_use]
                pub const fn checked_mul(self, rhs: Self) -> Option<Self> {
                    let Some(res) = self.get().checked_mul(rhs.get()) else {
                        return None;
                    };

                    if res <= Self::MAX.get() {
                        Some(Self(res))
                    } else {
                        None
                    }
                }

                #[must_use]
                pub const fn saturating_mul(self, rhs: Self) -> Self {
                    match self.checked_mul(rhs)  {
                        Some(res) => res,
                        None => Self::MAX,
                    }
                }

                #[must_use]
                pub const fn checked_div(self, rhs: Self) -> Option<Self> {
                    match self.get().checked_div(rhs.get())  {
                        Some(res) => Some(Self(res)),
                        None => None,
                    }
                }
            }

            impl FromStr for $type {
                type Err = ParseIntError;

                fn from_str(value: &str) -> Result<Self, Self::Err> {
                    value
                        .parse::<$inner>()
                        .map_err(ParseIntError::Parse)
                        .and_then(|value| value.try_into().map_err(ParseIntError::OutOfRange))
                }
            }

            impl Display for $type {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    Display::fmt(&self.get(), f)
                }
            }

            impl From<$type> for $inner {
                fn from(value: $type) -> Self {
                    value.get()
                }
            }

            impl From<$type> for $inner_signed {
                fn from(value: $type) -> Self {
                    value.get_signed()
                }
            }

            impl TryFrom<$inner> for $type {
                type Error = TryFromIntError;

                fn try_from(value: $inner) -> Result<Self, Self::Error> {
                    Self::new(value).ok_or(TryFromIntError)
                }
            }

            impl TryFrom<$inner_signed> for $type {
                type Error = TryFromIntError;

                fn try_from(value: $inner_signed) -> Result<Self, Self::Error> {
                    Self::new_signed(value).ok_or(TryFromIntError)
                }
            }

            impl FromSql<$sql_type, Pg> for $type {
                fn from_sql(bytes: PgValue<'_>) -> diesel::deserialize::Result<Self> {
                    let value = <$inner_signed as FromSql<$sql_type, Pg>>::from_sql(bytes)?;
                    Self::new_signed(value)
                        .ok_or_else(|| Box::new(TryFromIntError) as Box<dyn Error + Send + Sync + 'static>)
                }
            }

            impl ToSql<$sql_type, Pg> for $type {
                fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> diesel::serialize::Result {
                    <$inner_signed as ToSql<$sql_type, Pg>>::to_sql(self.get_signed_ref(), out)
                }
            }
        )*
    }
}

impl_numbers! {
    U15 => u16, i16, SmallInt,
    U31 => u32, i32, Integer,
    U63 => u64, i64, BigInt
}
