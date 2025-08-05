use std::io::Write as _;

use diesel::{
    pg::{Pg, PgValue},
    serialize::IsNull,
};
use serde::{Serialize, de::DeserializeOwned};

/// Allows using [`deserialize_as`] for benzina [`Json`] and [`Jsonb`] structs.
///
/// [`Jsonb`]: crate::Jsonb
/// [`Json`]: crate::Jsonb
/// [`deserialize_as`]: diesel::prelude::Queryable#deserialize_as-attribute
#[macro_export]
macro_rules! json_deserialize_as {
    (
        $($type:ty),*
    ) => {
        $(
            impl $crate::__private::std::convert::From<$crate::Jsonb<$type>> for $type {
                fn from(value: $crate::Jsonb<$type>) -> Self {
                    $crate::Jsonb::into_inner(value)
                }
            }

            impl $crate::__private::std::convert::From<$crate::Json<$type>> for $type {
                fn from(value: $crate::Json<$type>) -> Self {
                    $crate::Json::into_inner(value)
                }
            }
        )*
    };
}

pub(super) fn sql_serialize<T>(
    value: &T,
    out: &mut diesel::serialize::Output<'_, '_, Pg>,
) -> diesel::serialize::Result
where
    T: Serialize,
{
    serde_json::to_writer(out, value)
        .map(|()| IsNull::No)
        .map_err(Into::into)
}

pub(super) fn sql_serialize_binary<T>(
    value: &T,
    out: &mut diesel::serialize::Output<'_, '_, Pg>,
) -> diesel::serialize::Result
where
    T: Serialize,
{
    out.write_all(&[1])?;
    sql_serialize(value, out)
}

pub(super) fn sql_deserialize<T>(value: PgValue<'_>) -> diesel::deserialize::Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_slice(value.as_bytes()).map_err(Into::into)
}

pub(super) fn sql_deserialize_binary<T>(value: PgValue<'_>) -> diesel::deserialize::Result<T>
where
    T: DeserializeOwned,
{
    let (version, bytes) = value
        .as_bytes()
        .split_first()
        .ok_or("Empty JSONB payload")?;

    if *version != 1 {
        return Err("Unsupported JSONB encoding version".into());
    }

    serde_json::from_slice(bytes).map_err(Into::into)
}
