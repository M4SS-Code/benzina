use std::fmt::Debug;

use diesel::{
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    pg::{Pg, PgValue},
    serialize::{IsNull, ToSql},
    sql_types::Nullable,
};
use serde::{Serialize, de::DeserializeOwned};

macro_rules! impl_nullable {
    (
        $($type:ident => $diesel_type:ident => $uppercase_diesel_type:ident => $serializer:path => $deserializer:path => $diesel_type_import:path),*
    ) => {
        $(
            #[doc = concat!("A diesel [`Nullable`]<[`", stringify!($diesel_type), "`]> serialization and")]
            #[doc = "deserialization wrapper"]
            #[doc = ""]
            #[doc = concat!("Diesel only implements [`FromSql`] and [`ToSql`] for `Option`<[`serde_json::Value`]>, making")]
            #[doc = concat!("it hard to deal with _nullable_ `", stringify!($uppercase_diesel_type), "` columns. This type implements [`FromSql`] and [`ToSql`]")]
            #[doc = "for any type that implements [`Deserialize`] and [`Serialize`] respectively."]
            #[doc = ""]
            #[doc = "This type is not intended to be used directly in the model but rather to be used with diesel [`serialize_as`] and [`deserialize_as`]."]
            #[doc = "```"]
            #[doc = concat!("use benzina::{", stringify!($type), ", U31};")]
            #[doc = "use diesel::{Queryable, Insertable, sql_types::Nullable};"]
            #[doc = "use serde::{Deserialize, Serialize};"]
            #[doc = ""]
            #[doc = "#[derive(Debug, Queryable)]"]
            #[doc = "#[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]"]
            #[doc = "struct User {"]
            #[doc = "    id: U31,"]
            #[doc = "    first_name: String,"]
            #[doc = "    last_name: String,"]
            #[doc = concat!("    #[diesel(deserialize_as = ", stringify!($type), "<UserPermissions>)]")]
            #[doc = "    permissions: Option<UserPermissions>,"]
            #[doc = "}"]
            #[doc = ""]
            #[doc = "#[derive(Debug, Insertable)]"]
            #[doc = "#[diesel(table_name = users)]"]
            #[doc = "struct NewUser {"]
            #[doc = "    first_name: String,"]
            #[doc = "    last_name: String,"]
            #[doc = concat!("    #[diesel(serialize_as = ", stringify!($type),"<UserPermissions>)]")]
            #[doc = "    permissions: Option<UserPermissions>,"]
            #[doc = "}"]
            #[doc = ""]
            #[doc = "#[derive(Debug, Serialize, Deserialize)]"]
            #[doc = "struct UserPermissions {"]
            #[doc = "    can_delete: bool,"]
            #[doc = "    can_update: bool,"]
            #[doc = "    can_read: bool,"]
            #[doc = "}"]
            #[doc = ""]
            #[doc = "diesel::table! {"]
            #[doc = "    users (id) {"]
            #[doc = "        id -> Int4,"]
            #[doc = "        first_name -> Text,"]
            #[doc = "        last_name -> Text,"]
            #[doc = concat!("        permissions -> Nullable<", stringify!($diesel_type), ">,")]
            #[doc = "    }"]
            #[doc = "}"]
            #[doc = "```"]
            #[doc = "[`Nullable`]: diesel::sql_types::Nullable"]
            #[doc = concat!("[`", stringify!($diesel_type), "`]: diesel::sql_types::", stringify!($diesel_type))]
            #[doc = "[`FromSql`]: diesel::deserialize::FromSql"]
            #[doc = "[`ToSql`]: diesel::serialize::ToSql"]
            #[doc = "[`serde_json::Value`]: serde_json::Value"]
            #[doc = "[`Serialize`]: serde::Serialize"]
            #[doc = "[`Deserialize`]: serde::Deserialize"]
            #[doc = "[`serialize_as`]: diesel::prelude::Insertable#optional-field-attributes"]
            #[doc = "[`deserialize_as`]: diesel::prelude::Queryable#deserialize_as-attribute"]
            #[derive(
                Debug,
                Default,
                Clone,
                Copy,
                PartialEq,
                Eq,
                PartialOrd,
                Ord,
                Hash,
                FromSqlRow,
                AsExpression,
            )]
            #[diesel(sql_type = Nullable<$diesel_type_import>)]
            pub struct $type<T: Sized>(Option<T>);

            impl<T> $type<T> {
                pub const fn new(value: Option<T>) -> Self {
                    Self(value)
                }

                pub fn get(&self) -> Option<&T> {
                    self.0.as_ref()
                }

                pub fn into_inner(self) -> Option<T> {
                    self.0
                }
            }

            impl<T> From<$type<T>> for Option<T> {
                fn from(value: $type<T>) -> Self {
                    value.into_inner()
                }
            }

            impl<T> From<Option<T>> for $type<T> {
                fn from(value: Option<T>) -> Self {
                    Self::new(value)
                }
            }

            impl<T> FromSql<Nullable<$diesel_type_import>, Pg> for $type<T>
            where
                T: DeserializeOwned,
            {
                fn from_sql(value: PgValue) -> diesel::deserialize::Result<Self> {
                    $deserializer(value).map(Self)
                }

                fn from_nullable_sql(value: Option<PgValue>) -> diesel::deserialize::Result<Self> {
                    Ok(match value {
                        Some(bytes) => Self::from_sql(bytes)?,
                        None => Self(None),
                    })
                }
            }

            impl<T> ToSql<Nullable<$diesel_type_import>, Pg> for $type<T>
            where
                T: Debug + Serialize,
            {
                fn to_sql(&self, out: &mut diesel::serialize::Output<Pg>) -> diesel::serialize::Result {
                    if let Some(value) = &self.0 {
                        $serializer(value, out)
                    } else {
                        Ok(IsNull::Yes)
                    }
                }
            }
        )*
    };
}

impl_nullable!(
    NullableJson => Json => JSON => crate::json::convert::sql_serialize => crate::json::convert::sql_deserialize => diesel::sql_types::Json,
    NullableJsonb => Jsonb => JSONB => crate::json::convert::sql_serialize_binary => crate::json::convert::sql_deserialize_binary => diesel::pg::sql_types::Jsonb
);
