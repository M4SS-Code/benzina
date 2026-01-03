use proc_macro2::Span;
use quote::ToTokens;
use syn::{
    DeriveInput, Ident, Path, PathArguments, PathSegment, parse_macro_input,
    punctuated::Punctuated, token::PathSep,
};

use crate::join::Join;

use self::enum_derive::Enum;

mod enum_derive;
mod join;
mod rename_rule;

/// Derive [`FromSql`] and [`ToSql`] for a Rust enum.
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
/// # use benzina_derive as benzina;
/// # fn main() {}
///
/// #[derive(Debug, Copy, Clone, benzina::Enum)]
/// #[benzina(
///     sql_type = crate::schema::sql_types::Animal,
///     rename_all = "snake_case"
/// )]
/// # #[benzina(crate = fake_benzina)]
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
/// #
/// # mod fake_benzina {
/// #     pub mod __private {
/// #         pub use std;
/// #         pub use diesel;
/// #     }
/// # }
/// ```
///
/// ## Enums with variant-specific data in separate JSONB column
///
/// You can also use `benzina::Enum` for enums where each variant holds
/// associated data. This is useful when you have a PostgreSQL ENUM for the
/// discriminator and a JSONB column for the variant-specific payload.
///
/// ### migration
///
/// ```sql
/// CREATE TYPE animal AS ENUM ('chicken', 'duck', 'oca', 'rabbit');
///
/// CREATE TABLE pets (
///     id SERIAL PRIMARY KEY,
///     name TEXT NOT NULL,
///     animal animal NOT NULL,
///     animal_data JSONB NOT NULL
/// );
/// ```
///
/// ### Rust enum
///
/// ```rust
/// # use benzina_derive as benzina;
/// # fn main() {}
/// use diesel::pg::Pg;
/// use diesel::{Identifiable, Insertable, Queryable, Selectable};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Queryable, Identifiable, Insertable, Selectable)]
/// #[diesel(table_name = schema::pets, check_for_backend(Pg))]
/// pub struct Pet {
///     pub id: i32,
///     pub name: String,
///     #[diesel(embed)]
///     pub animal: Animal,
/// }
///
/// #[derive(Debug, Clone, benzina::Enum)]
/// #[benzina(
///     sql_type = schema::sql_types::Animal,
///     rename_all = "snake_case",
///     table = schema::pets,
///     column = animal,
///     data_column = animal_data
/// )]
/// # #[benzina(crate = fake_benzina)]
/// pub enum Animal {
///     Chicken(ChickenData),
///     Duck(DuckData),
///     #[benzina(rename = "oca")]
///     Goose(GooseData),
///     Rabbit(RabbitData),
/// }
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// pub struct ChickenData {
///     pub likes_cuddles: bool,
///     pub breed: String,
/// }
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// pub struct DuckData {
///     pub favorite_treat: String,
///     pub feather_color: String,
/// }
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// pub struct GooseData {
///     pub weight_kg: f64,
///     pub honks_at_strangers: bool,
/// }
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// pub struct RabbitData {
///     pub fur_color: String,
///     pub litter_trained: bool,
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
///
///     diesel::table! {
///         use diesel::sql_types::*;
///         use super::sql_types::Animal;
///
///         pets (id) {
///             id -> Int4,
///             name -> Text,
///             animal -> Animal,
///             animal_data -> Jsonb,
///         }
///     }
/// }
/// #
/// # mod fake_benzina {
/// #     pub mod __private {
/// #         pub use std;
/// #         pub use diesel;
/// #
/// #         pub mod json {
/// #             use diesel::{
/// #                 deserialize::{FromSql, FromSqlRow},
/// #                 expression::AsExpression,
/// #                 pg::{Pg, PgValue},
/// #                 serialize::ToSql,
/// #                 sql_types,
/// #             };
/// #             use serde::{Deserialize, Serialize};
/// #
/// #             #[derive(Debug, FromSqlRow, AsExpression)]
/// #             #[diesel(sql_type = sql_types::Jsonb)]
/// #             pub struct RawJsonb;
/// #
/// #             impl RawJsonb {
/// #                 pub const EMPTY: Self = Self;
/// #
/// #                 pub fn serialize(value: &impl Serialize) -> diesel::deserialize::Result<Self> {
/// #                     unimplemented!()
/// #                 }
/// #
/// #                 pub fn deserialize<T: for<'a> Deserialize<'a>>(&self) -> diesel::deserialize::Result<T> {
/// #                     unimplemented!()
/// #                 }
/// #             }
/// #
/// #             impl FromSql<sql_types::Jsonb, Pg> for RawJsonb {
/// #                 fn from_sql(value: PgValue) -> diesel::deserialize::Result<Self> {
/// #                     unimplemented!()
/// #                 }
/// #             }
/// #
/// #             impl ToSql<sql_types::Jsonb, Pg> for RawJsonb {
/// #                 fn to_sql(&self, out: &mut diesel::serialize::Output<Pg>) -> diesel::serialize::Result {
/// #                     unimplemented!()
/// #                 }
/// #             }
/// #         }
/// #     }
/// # }
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

/// Convert the output of a query containing joins into a properly nested structure.
///
/// <div class="warning">
///     This macro is still in the experimental stage and may contain
///     bugs and unhelpful error diagnostics.
/// </div>
///
/// Enable the `rustc-hash` feature to use a faster but non-DOS-resistant hasher for
/// the internal maps.
///
/// ## Example
///
/// ```rust,compile_fail
/// # fn main() {}
///
/// use diesel::{
///     Identifiable, QueryDsl, QueryResult, Queryable, RunQueryDsl, Selectable, SelectableHelper,
///     pg::{Pg, PgConnection},
/// };
///
/// #[derive(Debug, Queryable, Identifiable, Selectable)]
/// #[diesel(table_name = users, check_for_backend(Pg))]
/// pub struct User {
///     pub id: i32,
///     pub name: String,
/// }
///
/// #[derive(Debug)]
/// pub struct UserWithPosts {
///     pub user: User,
///     pub posts: Vec<PostFromUser>,
/// }
///
/// #[derive(Debug, Queryable, Identifiable, Selectable)]
/// #[diesel(table_name = topics, check_for_backend(Pg))]
/// pub struct Topic {
///     pub id: i32,
///     pub name: String,
/// }
///
/// #[derive(Debug, Queryable, Identifiable, Selectable)]
/// #[diesel(table_name = posts, check_for_backend(Pg))]
/// pub struct Post {
///     pub id: i32,
///     pub user_id: i32,
///     pub topic_id: i32,
///     pub message: String,
/// }
///
/// #[derive(Debug)]
/// pub struct PostFromUser {
///     pub post: Post,
///     pub topic: Topic,
///     pub comments: Vec<CommentFromPost>,
/// }
///
/// #[derive(Debug, Queryable, Identifiable, Selectable)]
/// #[diesel(table_name = comments, check_for_backend(Pg))]
/// pub struct Comment {
///     pub id: i32,
///     pub post_id: i32,
///     pub user_id: i32,
///     pub message: String,
/// }
///
/// #[derive(Debug)]
/// pub struct CommentFromPost {
///     pub comment: Comment,
///     pub user: User,
/// }
///
/// impl UserWithPosts {
///     pub fn get_by_id(conn: &mut PgConnection, user_id: i32) -> QueryResult<Vec<Self>> {
///         let (users1, users2) = diesel::alias!(users as users1, users as users2);
///
///         let records = users1
///             .find(user_id)
///             .left_join(
///                 posts::table
///                     .left_join(topics::table)
///                     .left_join(comments::table.left_join(users2)),
///             )
///             .select((
///                 users1.fields(<User as Selectable<Pg>>::construct_selection()),
///                 Option::<Post>::as_select(),
///                 Option::<Topic>::as_select(),
///                 Option::<Comment>::as_select(),
///                 users2.fields(<Option<User> as Selectable<Pg>>::construct_selection()),
///             ))
///             .get_results::<(
///                 User,
///                 Option<Post>,
///                 Option<Topic>,
///                 Option<Comment>,
///                 Option<User>,
///             )>(conn)?;
///
///         let joined = benzina::join! {
///             records,
///             Vec<UserWithPosts {
///                 user: One<0>,
///                 posts: Vec0<PostFromUser {
///                     post: One<1>,
///                     topic: AssumeOne<2>,
///                     comments: Vec0<CommentFromPost {
///                         comment: One<3>,
///                         user: AssumeOne<4>,
///                     }>,
///                 }>,
///             }>,
///         };
///         Ok(joined)
///     }
/// }
///
/// diesel::table! {
///     users {
///         id -> Integer,
///         name -> Text,
///     }
/// }
///
/// diesel::table! {
///     topics {
///         id -> Integer,
///         name -> Text,
///     }
/// }
///
/// diesel::table! {
///     posts {
///         id -> Integer,
///         user_id -> Integer,
///         topic_id -> Integer,
///         message -> Text,
///     }
/// }
///
/// diesel::table! {
///     comments {
///         id -> Integer,
///         post_id -> Integer,
///         user_id -> Integer,
///         message -> Text,
///     }
/// }
///
/// diesel::joinable!(posts -> users (user_id));
/// diesel::joinable!(posts -> topics (topic_id));
/// diesel::joinable!(comments -> posts (post_id));
/// diesel::joinable!(comments -> users (user_id));
///
/// diesel::allow_tables_to_appear_in_same_query!(users, topics, posts, comments);
/// ```
#[proc_macro]
pub fn join(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Join);
    input.into_token_stream().into()
}

#[expect(clippy::ref_option, reason = "it's easier to use")]
fn crate_name(crate_name: &Option<Path>) -> Path {
    crate_name.clone().unwrap_or_else(|| {
        let mut segments = Punctuated::new();
        segments.push(PathSegment {
            ident: Ident::new("benzina", Span::call_site()),
            arguments: PathArguments::None,
        });
        Path {
            leading_colon: Some(PathSep {
                spans: [Span::call_site(); 2],
            }),
            segments,
        }
    })
}
