#[cfg(feature = "postgres")]
pub use self::int::{U15, U31, U63};
#[cfg(feature = "derive")]
pub use benzina_derive::Enum;

#[cfg(feature = "postgres")]
pub mod error;
#[cfg(feature = "postgres")]
mod int;
#[cfg(all(feature = "serde", feature = "postgres"))]
mod serde;
#[cfg(all(feature = "utoipa", feature = "postgres"))]
mod utoipa;
