use std::{
    error::Error,
    fmt::{self, Display},
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
            #[doc = concat!("A positive [`", stringify!($inner_signed), "`]")]
            #[doc = ""]
            #[doc = "Represents the positive integer range that a"]
            #[doc = concat!("`", stringify!($sql_type), "` PostgreSQL column with")]
            #[doc = "a `>= 0` CHECK constraint is able to represent."]
            #[doc = ""]
            #[doc = concat!("This allows safe storage in PostgreSQL as ", stringify!($sql_type), " while maintaining")]
            #[doc = "non-negative semantics in Rust code."]
            #[doc = ""]
            #[doc = "# Examples"]
            #[doc = ""]
            #[doc = "```rust"]
            #[doc = concat!("use benzina::", stringify!($type), ";")]
            #[doc = ""]
            #[doc = concat!("let value = ", stringify!($type), "::new(100).unwrap();")]
            #[doc = concat!("assert_eq!(value.get(), 100);")]
            #[doc = "```"]
            #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[derive(FromSqlRow, AsExpression)]
            #[diesel(sql_type = $sql_type)]
            pub struct $type($inner);

            impl $type {
                /// The size of this integer type in bits.
                pub const BITS: u32 = $inner::BITS - 1;
                /// The smallest value that can be represented by this integer type.
                pub const MIN: Self = Self(0);
                /// The largest value that can be represented by this integer type.
                pub const MAX: Self = Self($inner::MAX >> 1);

                /// Creates a new value from an unsigned integer if it fits within the valid range.
                #[must_use]
                pub const fn new(n: $inner) -> Option<Self> {
                    if n <= Self::MAX.get() {
                        Some(Self(n))
                    } else {
                        None
                    }
                }

                /// Creates a new value from a signed integer if it fits within the valid range.
                #[expect(clippy::cast_sign_loss, reason = "we assert that `n` is positive")]
                #[must_use]
                pub const fn new_signed(n: $inner_signed) -> Option<Self> {
                    if n >= Self::MIN.get_signed() {
                        Some(Self(n as $inner))
                    } else {
                        None
                    }
                }

                /// Returns the value as an unsigned integer.
                #[must_use]
                pub const fn get(self) -> $inner {
                    self.0
                }

                /// Returns the value as a signed integer.
                #[expect(clippy::cast_possible_wrap, reason = "the number is in the positive range of the signed output")]
                #[must_use]
                pub const fn get_signed(self) -> $inner_signed {
                    self.get() as $inner_signed
                }

                /// Checked integer addition. Computes `self + rhs`, returning `None` if overflow occurred.
                #[expect(clippy::cast_sign_loss, reason = "`checked_add` of `$inner_signed` integers returns `$inner_signed` in range of `$inner`")]
                #[must_use]
                pub const fn checked_add(self, rhs: Self) -> Option<Self> {
                    let Some(res) = self.get_signed().checked_add(rhs.get_signed()) else {
                        return None;
                    };
                    Some(Self(res as $inner))
                }

                /// Saturating integer addition. Computes `self + rhs`, saturating at the numeric bounds instead of overflowing.
                #[must_use]
                pub const fn saturating_add(self, rhs: Self) -> Self {
                    match self.checked_add(rhs)  {
                        Some(res) => res,
                        None => Self::MAX,
                    }
                }

                /// Checked integer addition. Computes `self + rhs`, returning `None` if overflow occurred.
                #[must_use]
                pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
                    match self.get().checked_sub(rhs.get()) {
                        Some(res) => Some(Self(res)),
                        None => None,
                    }
                }

                /// Saturating integer subtraction. Computes `self - rhs`, saturating at the numeric bounds instead of overflowing.
                #[must_use]
                pub const fn saturating_sub(self, rhs: Self) -> Self {
                    match self.checked_sub(rhs)  {
                        Some(res) => res,
                        None => Self::MIN,
                    }
                }

                /// Checked integer multiplication. Computes `self * rhs`, returning `None` if overflow occurred.
                #[expect(clippy::cast_sign_loss, reason = "`checked_mul` of `$inner_signed` integers returns `$inner_signed` in range of `$inner`")]
                #[must_use]
                pub const fn checked_mul(self, rhs: Self) -> Option<Self> {
                    let Some(res) = self.get_signed().checked_mul(rhs.get_signed()) else {
                        return None;
                    };
                    Some(Self(res as $inner))
                }

                /// Saturating integer multiplication. Computes `self * rhs`, saturating at the numeric bounds instead of overflowing.
                #[must_use]
                pub const fn saturating_mul(self, rhs: Self) -> Self {
                    match self.checked_mul(rhs)  {
                        Some(res) => res,
                        None => Self::MAX,
                    }
                }

                /// Checked integer division. Computes `self / rhs`, returning `None` if `rhs == 0`.
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

            impl Default for $type {
                fn default() -> Self {
                    const { Self::new(0).unwrap() }
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
                    <$inner_signed as ToSql<$sql_type, Pg>>::to_sql(&self.get_signed(), &mut out.reborrow())
                }
            }
        )*
    }
}

macro_rules! from_numbers {
    ($($from:ident => $to:ident),*) => {
        $(
            impl From<$from> for $to {
                fn from(value: $from) -> Self {
                    Self(value.get().into())
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

from_numbers! {
    U15 => U31,
    U15 => U63,
    U31 => U63
}

#[cfg(test)]
mod tests {
    use super::{U15, U31, U63};

    #[test]
    fn test_constants() {
        assert_eq!(0, U15::MIN.get());
        assert_eq!(32767, U15::MAX.get()); // 2^15 - 1
        assert_eq!(15, U15::BITS);

        assert_eq!(0, U31::MIN.get());
        assert_eq!(2147483647, U31::MAX.get()); // 2^31 - 1
        assert_eq!(31, U31::BITS);

        assert_eq!(0, U63::MIN.get());
        assert_eq!(9223372036854775807, U63::MAX.get()); // 2^63 - 1
        assert_eq!(63, U63::BITS);
    }

    #[test]
    fn test_new() {
        assert!(U15::new(0).is_some());
        assert!(U15::new(32767).is_some());
        assert!(U15::new(32768).is_none());
        assert!(U15::new(u16::MAX).is_none());

        assert!(U31::new(0).is_some());
        assert!(U31::new(2147483647).is_some());
        assert!(U31::new(2147483648).is_none());
        assert!(U31::new(u32::MAX).is_none());

        assert!(U63::new(0).is_some());
        assert!(U63::new(9223372036854775807).is_some());
        assert!(U63::new(9223372036854775808).is_none());
        assert!(U63::new(u64::MAX).is_none());
    }

    #[test]
    fn test_new_signed() {
        assert!(U15::new_signed(-1).is_none());
        assert!(U15::new_signed(0).is_some());
        assert!(U15::new_signed(i16::MAX).is_some());

        assert!(U31::new_signed(-1).is_none());
        assert!(U31::new_signed(0).is_some());
        assert!(U31::new_signed(i32::MAX).is_some());

        assert!(U63::new_signed(-1).is_none());
        assert!(U63::new_signed(0).is_some());
        assert!(U63::new_signed(i64::MAX).is_some());
    }

    #[test]
    fn test_get_methods() {
        let val = U15::new(1000).unwrap();
        assert_eq!(1000u16, val.get());
        assert_eq!(1000i16, val.get_signed());
    }

    #[test]
    fn test_checked_arithmetic() {
        let a = U15::new(100).unwrap();
        let b = U15::new(200).unwrap();

        // Addition
        assert_eq!(Some(U15::new(300).unwrap()), a.checked_add(b));
        assert_eq!(None, U15::MAX.checked_add(U15::new(1).unwrap()));

        // Subtraction
        assert_eq!(Some(U15::new(100).unwrap()), b.checked_sub(a));
        assert_eq!(None, a.checked_sub(b));

        // Multiplication
        assert_eq!(
            Some(U15::new(200).unwrap()),
            a.checked_mul(U15::new(2).unwrap())
        );
        assert_eq!(None, U15::MAX.checked_mul(U15::new(2).unwrap()));

        // Division
        assert_eq!(Some(U15::new(2).unwrap()), b.checked_div(a));
        assert_eq!(None, a.checked_div(U15::new(0).unwrap()));
    }

    #[test]
    fn test_saturating_arithmetic() {
        let a = U15::new(100).unwrap();
        let b = U15::new(200).unwrap();

        // Addition
        assert_eq!(U15::new(300).unwrap(), a.saturating_add(b));
        assert_eq!(U15::MAX, U15::MAX.saturating_add(U15::new(1).unwrap()));

        // Subtraction
        assert_eq!(U15::new(100).unwrap(), b.saturating_sub(a));
        assert_eq!(U15::MIN, a.saturating_sub(b));

        // Multiplication
        assert_eq!(
            U15::new(200).unwrap(),
            a.saturating_mul(U15::new(2).unwrap())
        );
        assert_eq!(U15::MAX, U15::MAX.saturating_mul(U15::new(2).unwrap()));
    }

    #[test]
    fn test_string_parsing() {
        assert_eq!(U15::new(123).unwrap(), "123".parse::<U15>().unwrap());
        assert_eq!(
            U31::new(1000000).unwrap(),
            "1000000".parse::<U31>().unwrap()
        );
        assert_eq!(
            U63::new(9223372036854775807).unwrap(),
            "9223372036854775807".parse::<U63>().unwrap()
        );

        assert!("32768".parse::<U15>().is_err()); // Out of range
        assert!("-1".parse::<U15>().is_err()); // Negative
        assert!("abc".parse::<U15>().is_err()); // Invalid format
    }

    #[test]
    fn test_display() {
        assert_eq!("42", U15::new(42).unwrap().to_string());
    }

    #[test]
    fn test_conversions() {
        let val = U15::new(1000).unwrap();

        // From U15
        assert_eq!(1000, u16::from(val));
        assert_eq!(1000, i16::from(val));

        // To U15
        assert_eq!(U15::new(1000).unwrap(), U15::try_from(1000u16).unwrap());
        assert_eq!(U15::new(1000).unwrap(), U15::try_from(1000i16).unwrap());
        assert!(U15::try_from(40000u16).is_err());
        assert!(U15::try_from(-1i16).is_err());
    }

    #[test]
    fn test_ordering() {
        let a = U15::new(100).unwrap();
        let b = U15::new(200).unwrap();
        let c = U15::new(100).unwrap();

        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, c);
        assert!(a <= c);
        assert!(a >= c);
    }
}
