use quote::ToTokens;
use syn::{DeriveInput, parse_macro_input};

use self::enum_derive::Enum;

mod enum_derive;
mod rename_rule;

/// Derive [`FromSql`] and [`ToSql`] for a Rust enum.
#[expect(clippy::doc_markdown, reason = "this is not a Rust type")]
/// Represents a PostgreSQL enum as a Rust enum.
///
/// ## Example
///
/// ### migration
///
/// ```sql
/// CREATE TYPE animal AS ENUM (
///     'chicken',
///     'duck',
///     'oca',
///     'rabbit
/// );
/// ```
///
/// ### Rust enum
///
/// ```rust
/// # fn main() {}
///
/// #[derive(Debug, Copy, Clone, benzina::Enum)]
/// #[benzina(
///     sql_type = "crate::schema::sql_types::Animal",
///     rename_all = "snake_case"
/// )]
/// pub enum Animal {
///     Chicken,
///     Duck,
///     #[benzina(rename = "oca")]
///     Goose,
///     Rabbit,
/// }
///
/// pub mod schema {
///     // @generated automatically by Diesel CLI.
///
///     pub mod sql_types {
///         #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
///         #[diesel(postgres_type(name = "animal"))]
///         pub struct Animal;
///     }
/// }
/// ```
///
/// [`FromSql`]: https://docs.rs/diesel/latest/diesel/deserialize/trait.FromSql.html
/// [`ToSql`]: https://docs.rs/diesel/latest/diesel/serialize/trait.ToSql.html
#[proc_macro_derive(Enum, attributes(benzina))]
pub fn benzina_enum_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    Enum::parse(input)
        .map_or_else(syn::Error::into_compile_error, ToTokens::into_token_stream)
        .into()
}
