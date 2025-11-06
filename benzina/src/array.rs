use std::fmt::Debug;

use diesel::{
    deserialize::{FromSql, FromSqlRow},
    expression::{AppearsOnTable, Expression, SelectableExpression},
    pg::{Pg, PgValue},
    query_builder::{AstPass, QueryFragment, QueryId},
    result::QueryResult,
    serialize::ToSql,
    sql_types::{self, BigInt, Bool, Double, Float, Integer, Nullable, SmallInt, Text},
};

use crate::{U15, U31, U63, error::InvalidArray};

/// A diesel [`Array`] serialization and deserialization wrapper __without__ NULL items
///
/// Since postgres ignores the array dimension (if specified), diesel implements
/// [`FromSql`] for `Vec<T>` and [`ToSql`] for `Vec<T>`/`&[T]`.
/// In addition postgres also considers array items as always nullable.
/// This makes it hard to deal with real arrays that have a predetermined length
/// and an homogeneous nullability.
/// This type checks at runtime the array length and the __non__ nullability of its items,
/// therefore to be completely safe, you should also add the following `CHECK` constraints:
/// ```sql
/// array_ndims(array_field) = 1 AND
/// array_length(array_field, 1) = N AND
/// array_position(array_field, NULL) IS NULL
/// ```
///
/// This type is not intended to be used directly in the model but rather to be
/// used with diesel [`serialize_as`] and [`deserialize_as`].
///
/// ```
/// use benzina::{Array, U31};
/// use diesel::{Insertable, Queryable};
///
/// #[derive(Debug, Queryable)]
/// #[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]
/// struct User {
///     id: U31,
///     first_name: String,
///     last_name: String,
///     #[diesel(deserialize_as = Array<bool, 5>)]
///     flags: UserFlags,
/// }
///
/// #[derive(Debug, Insertable)]
/// #[diesel(table_name = users)]
/// struct NewUser {
///     first_name: String,
///     last_name: String,
///     #[diesel(serialize_as = Array<bool, 5>)]
///     flags: UserFlags,
/// }
///
/// #[derive(Debug)]
/// struct UserFlags([bool; 5]);
///
/// // needed by deserialize_as
/// impl From<benzina::Array<bool, 5>> for UserFlags  {
///    fn from(value: benzina::Array<bool, 5>) -> Self {
///        Self(value.into_inner())
///    }
/// }
///
/// // needed by serialize_as
/// impl From<UserFlags> for benzina::Array<bool, 5> {
///    fn from(value: UserFlags) -> Self {
///        Self::new(value.0)
///    }
/// }
///
/// diesel::table! {
///     users (id) {
///         id -> Int4,
///         first_name -> Text,
///         last_name -> Text,
///         flags -> Array<Nullable<Bool>>,
///     }
/// }
/// ```
///
/// [`Array`]: diesel::sql_types::Array
/// [`serialize_as`]: diesel::prelude::Insertable#optional-field-attributes
/// [`deserialize_as`]: diesel::prelude::Queryable#deserialize_as-attribute
#[derive(Debug, FromSqlRow)]
pub struct Array<T, const N: usize>([T; N]);
impl<T, const N: usize> Array<T, N> {
    #[must_use]
    pub fn new(values: [T; N]) -> Self {
        Self(values)
    }

    #[must_use]
    pub fn into_inner(self) -> [T; N] {
        self.0
    }
}

/// A diesel [`Array`](diesel::sql_types::Array) serialization and deserialization wrapper __with__ NULL items
///
/// This type works exactly as benzina [`Array`](crate::Array), with the following execeptions:
/// - it does not require __non__ nullable items
/// - it shouldn't be used with the following nullability `CHECK` constraint:
///   ```sql
///   array_position(array_field, NULL) IS NULL
///   ```
#[derive(Debug, FromSqlRow)]
pub struct ArrayWithNullableItems<T, const N: usize>([Option<T>; N]);
impl<T, const N: usize> ArrayWithNullableItems<T, N> {
    #[must_use]
    pub fn new(values: [Option<T>; N]) -> Self {
        Self(values)
    }

    #[must_use]
    pub fn into_inner(self) -> [Option<T>; N] {
        self.0
    }
}

macro_rules! impl_array {
    (
        $(
            $rust_type:ident => $diesel_type:ident
        ),*
    ) => {
        $(
            impl<const N: usize> Expression for Array<$rust_type, N> {
                type SqlType = sql_types::Array<Nullable<$diesel_type>>;
            }

            impl< const N: usize> QueryId for Array<$rust_type, N> {
                type QueryId = <sql_types::Array<Nullable<$diesel_type>> as QueryId>::QueryId;

                const HAS_STATIC_QUERY_ID: bool = <sql_types::Array<Nullable<$diesel_type>> as QueryId>::HAS_STATIC_QUERY_ID;
            }

            impl<const N: usize> QueryFragment<Pg> for Array<$rust_type, N>
            {
                fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
                    pass.push_bind_param(self)?;
                    Ok(())
                }
            }

            impl<__QS, const N: usize> AppearsOnTable<__QS> for Array<$rust_type, N> {}

            impl<__QS, const N: usize> SelectableExpression<__QS> for Array<$rust_type, N> {}

            impl<const N: usize> ToSql<sql_types::Array<Nullable<$diesel_type>>, Pg> for Array<$rust_type, N>
            {
                fn to_sql<'b>(
                    &'b self,
                    out: &mut diesel::serialize::Output<'b, '_, Pg>,
                ) -> diesel::serialize::Result {
                    <[$rust_type] as ToSql<sql_types::Array<$diesel_type>, Pg>>::to_sql(&self.0.as_slice(), out)
                }
            }

            impl<const N: usize> FromSql<sql_types::Array<Nullable<$diesel_type>>, Pg> for Array<$rust_type, N>
            {
                fn from_sql(bytes: PgValue<'_>) -> diesel::deserialize::Result<Self> {
                    let raw = <Vec<Option<$rust_type>> as FromSql<sql_types::Array<Nullable<$diesel_type>>, Pg>>::from_sql(bytes)?;

                    let res: [$rust_type; N] = raw
                        .into_iter()
                        .collect::<Option<Vec<$rust_type>>>()
                        .ok_or(diesel::result::Error::DeserializationError(Box::new(
                            InvalidArray::UnexpectedNullValue,
                        )))?
                        .try_into()
                        .map_err(|_| {
                            diesel::result::Error::DeserializationError(Box::new(
                                InvalidArray::UnexpectedLength,
                            ))
                        })?;

                    Ok(Self(res))
                }
            }
        )*
    }
}

macro_rules! impl_array_with_nullable_items {
    (
        $(
            $rust_type:ident => $diesel_type:ident
        ),*
    ) => {
        $(
            impl<const N: usize> Expression for ArrayWithNullableItems<$rust_type, N> {
                type SqlType = sql_types::Array<Nullable<$diesel_type>>;
            }

            impl<const N: usize> QueryId for ArrayWithNullableItems<$rust_type, N> {
                type QueryId = <sql_types::Array<Nullable<$diesel_type>> as QueryId>::QueryId;

                const HAS_STATIC_QUERY_ID: bool = <sql_types::Array<Nullable<$diesel_type>> as QueryId>::HAS_STATIC_QUERY_ID;
            }

            impl<const N: usize> QueryFragment<Pg> for ArrayWithNullableItems<$rust_type, N>
            {
                fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
                    pass.push_bind_param(self)?;
                    Ok(())
                }
            }

            impl<__QS, const N: usize> AppearsOnTable<__QS> for ArrayWithNullableItems<$rust_type, N> {}
            impl<__QS, const N: usize> SelectableExpression<__QS> for ArrayWithNullableItems<$rust_type, N> {}


            impl<const N: usize> ToSql<sql_types::Array<Nullable<$diesel_type>>, Pg> for ArrayWithNullableItems<$rust_type, N>
            {
                fn to_sql<'b>(
                    &'b self,
                    out: &mut diesel::serialize::Output<'b, '_, Pg>,
                ) -> diesel::serialize::Result {
                    <[Option<$rust_type>] as ToSql<sql_types::Array<Nullable<$diesel_type>>, Pg>>::to_sql(self.0.as_slice(), out)
                }
            }

            impl<const N: usize> FromSql<sql_types::Array<Nullable<$diesel_type>>, Pg> for ArrayWithNullableItems<$rust_type, N>
            {
                fn from_sql(bytes: PgValue<'_>) -> diesel::deserialize::Result<Self> {
                    let raw = <Vec<Option<$rust_type>> as FromSql<sql_types::Array<Nullable<$diesel_type>>, Pg>>::from_sql(bytes)?;

                    let res: [Option<$rust_type>; N] = raw
                        .try_into()
                        .map_err(|_| {
                            diesel::result::Error::DeserializationError(Box::new(
                                InvalidArray::UnexpectedLength,
                            ))
                        })?;

                    Ok(Self(res))
                }
            }
        )*
    };
}

impl_array! {
    U15 => SmallInt,
    U31 => Integer,
    U63 => BigInt,
    i16 => SmallInt,
    i32 => Integer,
    i64 => BigInt,
    f32 => Float,
    f64 => Double,
    bool => Bool,
    String => Text
}

impl_array_with_nullable_items! {
    U15 => SmallInt,
    U31 => Integer,
    U63 => BigInt,
    i16 => SmallInt,
    i32 => Integer,
    i64 => BigInt,
    f32 => Float,
    f64 => Double,
    bool => Bool,
    String => Text
}
