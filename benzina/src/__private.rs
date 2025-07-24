#[cfg(any(feature = "derive", feature = "typed-uuid"))]
pub use ::diesel;
#[cfg(feature = "derive")]
pub use ::indexmap;
#[cfg(feature = "serde")]
pub use ::serde;
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
