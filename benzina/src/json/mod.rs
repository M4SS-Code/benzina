use std::fmt::Debug;

use diesel::{
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    pg::{Pg, PgValue},
    serialize::ToSql,
    sql_types,
};
use serde::{Serialize, de::DeserializeOwned};

use crate::json::convert::{sql_deserialize, sql_serialize};

pub(crate) mod binary;
pub(crate) mod convert;
pub(crate) mod nullable;

/// A diesel [`Json`] serialization and deserialization
/// wrapper
///
/// Diesel only implements [`FromSql`] and [`ToSql`] for [`serde_json::Value`],
/// making it hard to deal with `JSON` columns. This type implements [`FromSql`]
/// and [`ToSql`] for any type that implements [`Deserialize`] and [`Serialize`]
/// respectively.
///
/// This type is not intended to be used directly in the model but rather to be
/// used with diesel [`serialize_as`] and [`deserialize_as`].
///
/// To use [`serialize_as`] you _MUST_ use [`json_deserialize_as`].
/// ```
/// use benzina::{Json, U31, json_deserialize_as};
/// use diesel::{Insertable, Queryable};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Queryable)]
/// #[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]
/// struct User {
///     id: U31,
///     first_name: String,
///     last_name: String,
///     #[diesel(deserialize_as = Json<UserPermissions>)]
///     permissions: UserPermissions,
/// }
///
/// #[derive(Debug, Insertable)]
/// #[diesel(table_name = users)]
/// struct NewUser {
///     first_name: String,
///     last_name: String,
///     #[diesel(serialize_as = Json<UserPermissions>)]
///     permissions: UserPermissions,
/// }
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct UserPermissions {
///     can_delete: bool,
///     can_update: bool,
///     can_read: bool,
/// }
///
/// diesel::table! {
///     users (id) {
///         id -> Int4,
///         first_name -> Text,
///         last_name -> Text,
///         permissions -> Json,
///     }
/// }
///
/// // It is NECESSARY to use deserialize_as
/// json_deserialize_as!(UserPermissions);
/// ```
///
/// [`Json`]: diesel::sql_types::Json
/// [`FromSql`]: diesel::deserialize::FromSql
/// [`ToSql`]: diesel::serialize::ToSql
/// [`serde_json::Value`]: serde_json::Value
/// [`Serialize`]: serde::Serialize
/// [`Deserialize`]: serde::Deserialize
/// [`serialize_as`]: diesel::prelude::Insertable#optional-field-attributes
/// [`deserialize_as`]: diesel::prelude::Queryable#deserialize_as-attribute
/// [`json_deserialize_as`]: crate::json_deserialize_as
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, FromSqlRow, AsExpression,
)]
#[diesel(sql_type = sql_types::Json)]
pub struct Json<T: Sized>(T);

impl<T> Json<T> {
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

impl<T> From<T> for Json<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> AsRef<T> for Json<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> FromSql<sql_types::Json, Pg> for Json<T>
where
    T: DeserializeOwned,
{
    fn from_sql(value: PgValue<'_>) -> diesel::deserialize::Result<Self> {
        sql_deserialize(value).map(Self)
    }
}

impl<T> ToSql<sql_types::Json, Pg> for Json<T>
where
    T: Debug + Serialize,
{
    fn to_sql(&self, out: &mut diesel::serialize::Output<Pg>) -> diesel::serialize::Result {
        sql_serialize(&self.0, out)
    }
}
