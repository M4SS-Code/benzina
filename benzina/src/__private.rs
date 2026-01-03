#[cfg(any(feature = "derive", feature = "typed-uuid"))]
pub use ::diesel;
#[cfg(feature = "derive")]
pub use ::indexmap;
#[cfg(feature = "serde")]
pub use ::serde_core;
pub use ::std;
#[cfg(feature = "typed-uuid")]
pub use ::uuid;

#[cfg(all(feature = "derive", feature = "rustc-hash"))]
type Hasher = rustc_hash::FxBuildHasher;

#[cfg(all(feature = "derive", not(feature = "rustc-hash")))]
type Hasher = std::hash::RandomState;

#[cfg(feature = "derive")]
pub type IndexMap<K, V> = indexmap::IndexMap<K, V, Hasher>;

#[cfg(feature = "derive")]
#[must_use]
pub fn new_indexmap<K, V>() -> IndexMap<K, V> {
    IndexMap::with_hasher(Hasher::default())
}

#[cfg(all(feature = "postgres", feature = "json"))]
pub mod json {
    use std::borrow::Cow;

    use diesel::{
        deserialize::{FromSql, FromSqlRow},
        expression::AsExpression,
        pg::{Pg, PgValue},
        serialize::ToSql,
        sql_types,
    };
    use serde_core::{Deserialize, Serialize};

    use crate::json::convert::{sql_deserialize_binary_raw, sql_serialize_binary_raw};

    #[derive(Debug, FromSqlRow, AsExpression)]
    #[diesel(sql_type = sql_types::Jsonb)]
    pub struct RawJsonb(Cow<'static, [u8]>);

    impl RawJsonb {
        pub const EMPTY: Self = Self(Cow::Borrowed(b"{}"));

        pub fn serialize(value: &impl Serialize) -> diesel::deserialize::Result<Self> {
            serde_json::to_vec(value)
                .map(Cow::Owned)
                .map(Self)
                .map_err(Into::into)
        }

        pub fn deserialize<T: for<'a> Deserialize<'a>>(&self) -> diesel::deserialize::Result<T> {
            serde_json::from_slice(&self.0).map_err(Into::into)
        }
    }

    impl FromSql<sql_types::Jsonb, Pg> for RawJsonb {
        fn from_sql(value: PgValue) -> diesel::deserialize::Result<Self> {
            sql_deserialize_binary_raw(&value)
                .map(ToOwned::to_owned)
                .map(Cow::Owned)
                .map(Self)
        }
    }

    impl ToSql<sql_types::Jsonb, Pg> for RawJsonb {
        fn to_sql(&self, out: &mut diesel::serialize::Output<Pg>) -> diesel::serialize::Result {
            sql_serialize_binary_raw(&self.0, out)
        }
    }
}

pub mod deep_clone {
    pub trait DeepClone {
        type Output;

        fn deep_clone(&self) -> Self::Output;
    }

    impl<T: Clone> DeepClone for &T {
        type Output = T;

        fn deep_clone(&self) -> Self::Output {
            (*self).clone()
        }
    }

    #[rustfmt::skip]
    mod impls {
        use super::DeepClone;

        macro_rules! impl_deep_clone_for_tuples {
            ($(($T:ident, $idx:tt)),+) => {
                impl<$($T: DeepClone),+> DeepClone for ($($T),+,) {
                    type Output = ($(<$T as DeepClone>::Output),+,);

                    fn deep_clone(&self) -> Self::Output {
                        ($((&self).$idx.deep_clone()),+,)
                    }
                }
            };
        }

        impl_deep_clone_for_tuples!((T1, 0));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1), (T3, 2));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1), (T3, 2), (T4, 3));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9), (T11, 10));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9), (T11, 10), (T12, 11));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9), (T11, 10), (T12, 11), (T13, 12));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9), (T11, 10), (T12, 11), (T13, 12), (T14, 13));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9), (T11, 10), (T12, 11), (T13, 12), (T14, 13), (T15, 14));
        impl_deep_clone_for_tuples!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9), (T11, 10), (T12, 11), (T13, 12), (T14, 13), (T15, 14), (T16, 15));
    }
}
