use diesel::{
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    pg::{Pg, PgValue},
    serialize::{Output, ToSql},
    sql_types::Binary as SqlBinary,
};

use crate::error::InvalidBinary;

/// A diesel serialization and deserialization wrapper for a fixed-length
/// PostgreSQL [`bytea`](diesel::sql_types::Binary) column.
///
/// Wraps an `[u8; N]` and checks, at runtime, that the stored/loaded blob is
/// exactly `N` bytes long. This makes it possible to model a `bytea` column
/// whose length is known at compile time, instead of relying on a loosely-typed
/// `Vec<u8>`.
///
/// To be completely safe, you should also add the following `CHECK` constraint:
/// ```sql
/// length(bytea_field) = N
/// ```
///
/// This type is not intended to be used directly in the model but rather to be
/// used with diesel [`serialize_as`] and [`deserialize_as`].
///
/// ```
/// use benzina::{Binary, U63};
/// use diesel::{Insertable, Queryable};
///
/// #[derive(Debug, Queryable)]
/// #[diesel(table_name = objects, check_for_backend(diesel::pg::Pg))]
/// struct Object {
///     id: U63,
///     // A SHA-256 digest of the object's content (always 32 bytes)
///     #[diesel(deserialize_as = Binary<32>)]
///     content_sha256: ObjectDigest,
/// }
///
/// #[derive(Debug, Insertable)]
/// #[diesel(table_name = objects)]
/// struct NewObject {
///     // A SHA-256 digest of the object's content (always 32 bytes)
///     #[diesel(serialize_as = Binary<32>)]
///     content_sha256: ObjectDigest,
/// }
///
/// /// A SHA-256 content digest (always 32 bytes).
/// #[derive(Debug)]
/// struct ObjectDigest([u8; 32]);
///
/// // needed by deserialize_as
/// impl From<benzina::Binary<32>> for ObjectDigest {
///     fn from(value: benzina::Binary<32>) -> Self {
///         Self(value.into_inner())
///     }
/// }
///
/// // needed by serialize_as
/// impl From<ObjectDigest> for benzina::Binary<32> {
///     fn from(value: ObjectDigest) -> Self {
///         benzina::Binary::new(value.0)
///     }
/// }
///
/// diesel::table! {
///     objects (id) {
///         id -> Int8,
///         content_sha256 -> Bytea,
///     }
/// }
/// ```
///
/// [`serialize_as`]: diesel::prelude::Insertable#optional-field-attributes
/// [`deserialize_as`]: diesel::prelude::Queryable#deserialize_as-attribute
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, FromSqlRow, AsExpression)]
#[diesel(sql_type = SqlBinary)]
pub struct Binary<const N: usize>([u8; N]);

impl<const N: usize> Binary<N> {
    /// Creates a new [`Binary<N>`] from the given fixed-length byte array.
    #[must_use]
    pub const fn new(values: [u8; N]) -> Self {
        Self(values)
    }

    /// Returns the inner fixed-length byte array by value.
    #[must_use]
    pub const fn into_inner(self) -> [u8; N] {
        self.0
    }

    /// Returns a reference to the inner fixed-length byte array.
    #[must_use]
    pub const fn get(&self) -> &[u8; N] {
        &self.0
    }

    /// Returns a slice over the inner bytes.
    #[must_use]
    pub const fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl<const N: usize> ToSql<SqlBinary, Pg> for Binary<N> {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> diesel::serialize::Result {
        <[u8] as ToSql<SqlBinary, Pg>>::to_sql(&self.0, out)
    }
}

impl<const N: usize> FromSql<SqlBinary, Pg> for Binary<N> {
    fn from_sql(bytes: PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let raw = <Vec<u8> as FromSql<SqlBinary, Pg>>::from_sql(bytes)?;

        let res: [u8; N] = raw.try_into().map_err(|_| {
            diesel::result::Error::DeserializationError(Box::new(InvalidBinary::UnexpectedLength))
        })?;

        Ok(Self(res))
    }
}
