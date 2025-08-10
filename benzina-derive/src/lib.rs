use quote::ToTokens;
use syn::{DeriveInput, parse_macro_input};

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
/// use diesel::{
///     deserialize::FromSqlRow,
///     expression::AsExpression,
/// };
///
/// #[derive(Debug, Copy, Clone, AsExpression, FromSqlRow, benzina::Enum)]
/// #[diesel(sql_type = crate::schema::sql_types::Animal)]
/// #[benzina(
///     sql_type = crate::schema::sql_types::Animal,
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
