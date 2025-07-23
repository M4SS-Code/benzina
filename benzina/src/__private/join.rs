pub use ::indexmap;

#[cfg(feature = "rustc-hash")]
type Hasher = rustc_hash::FxBuildHasher;

#[cfg(not(feature = "rustc-hash"))]
type Hasher = std::hash::RandomState;

pub type IndexMap<K, V> = indexmap::IndexMap<K, V, Hasher>;

pub fn new_indexmap<K, V>() -> IndexMap<K, V> {
    IndexMap::with_hasher(Hasher::default())
}
