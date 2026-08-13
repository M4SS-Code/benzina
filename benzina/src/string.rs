use std::fmt::Debug;

use diesel::{
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    sql_types,
};

/// Allows using [`deserialize_as`] for benzina [`Str`] structs.
///
/// [`deserialize_as`]: diesel::prelude::Queryable#deserialize_as-attribute
#[macro_export]
macro_rules! str_deserialize_as {
    (
        $($type:ty),*
    ) => {
        $(
            impl $crate::__private::std::convert::From<$crate::Str<$type>> for $type {
                fn from(value: $crate::Str<$type>) -> Self {
                    $crate::Str::into_inner(value)
                }
            }
        )*
    };
}

/// A diesel wrapper for types that implement [`FromStr`] and [`Display`]
/// for use with `Text` columns.
///
/// Diesel only implements [`FromSql`] and [`ToSql`] for [`String`],
/// making it hard to use custom types with `Text` columns. This type
/// implements [`FromSql`] for any type that implements [`FromStr`]
/// and [`ToSql`] for any type that implements [`Display`].
///
/// This type is not intended to be used directly in the model but rather to be
/// used with diesel [`serialize_as`] and [`deserialize_as`].
///
/// To use [`deserialize_as`] you _MUST_ use [`str_deserialize_as`].
///
/// ```
/// # use std::{fmt::{self, Display}, str::FromStr};
/// #
/// use benzina::{Str, U31, str_deserialize_as};
/// use diesel::{Insertable, Queryable};
///
/// #[derive(Debug, Queryable)]
/// #[diesel(table_name = pets, check_for_backend(diesel::pg::Pg))]
/// struct Pet {
///     id: U31,
///     name: String,
///     #[diesel(deserialize_as = Str<Animal>)]
///     animal: Animal,
/// }
///
/// #[derive(Debug, Insertable)]
/// #[diesel(table_name = pets)]
/// struct NewPet {
///     name: String,
///     #[diesel(serialize_as = Str<Animal>)]
///     animal: Animal,
/// }
///
/// #[derive(Debug)]
/// enum Animal {
///     Chicken,
///     Duck,
///     Goose,
///     Rabbit,
/// }
/// str_deserialize_as!(Animal);
///
/// impl Display for Animal {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         f.write_str(match self {
///             Self::Chicken => "chicken",
///             Self::Duck => "duck",
///             Self::Goose => "goose",
///             Self::Rabbit => "rabbit",
///         })
///     }
/// }
///
/// impl FromStr for Animal {
///     type Err = String;
///
///     fn from_str(s: &str) -> Result<Self, Self::Err> {
///         match s {
///             "chicken" => Ok(Self::Chicken),
///             "duck" => Ok(Self::Duck),
///             "goose" => Ok(Self::Goose),
///             "rabbit" => Ok(Self::Rabbit),
///             other => Err(format!("unknown animal: {other}")),
///         }
///     }
/// }
///
/// diesel::table! {
///     pets (id) {
///         id -> Int4,
///         name -> Text,
///         animal -> Text,
///     }
/// }
/// ```
///
/// [`FromSql`]: diesel::deserialize::FromSql
/// [`ToSql`]: diesel::serialize::ToSql
/// [`serialize_as`]: diesel::prelude::Insertable#optional-field-attributes
/// [`deserialize_as`]: diesel::prelude::Queryable#deserialize_as-attribute
/// [`str_deserialize_as`]: crate::str_deserialize_as
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, FromSqlRow, AsExpression,
)]
#[diesel(sql_type = sql_types::Text)]
pub struct Str<T: Sized>(T);

impl<T> Str<T> {
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    pub fn get(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for Str<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> AsRef<T> for Str<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

#[cfg(feature = "postgres")]
impl<T> FromSql<sql_types::Text, diesel::pg::Pg> for Str<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    fn from_sql(value: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let s = std::str::from_utf8(value.as_bytes())?;
        s.parse::<T>().map(Self).map_err(|e| e.to_string().into())
    }
}

#[cfg(feature = "postgres")]
impl<T> diesel::serialize::ToSql<sql_types::Text, diesel::pg::Pg> for Str<T>
where
    T: std::fmt::Display + Debug,
{
    fn to_sql(
        &self,
        out: &mut diesel::serialize::Output<diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        use std::io::Write;
        write!(out, "{}", self.0)?;
        Ok(diesel::serialize::IsNull::No)
    }
}
