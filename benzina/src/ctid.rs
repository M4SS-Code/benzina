use std::marker::PhantomData;

use diesel::{
    AppearsOnTable, Column, Expression, QueryResult, SelectableExpression, Table,
    expression::{ValidGrouping, is_aggregate::No},
    internal::table_macro::{StaticQueryFragment, StaticQueryFragmentInstance},
    pg::Pg,
    query_builder::{AstPass, QueryFragment, QueryId},
    query_source::{AppearsInFromClause, Once},
};

use crate::sql_types::Tid;

// EXPERIMENTAL: not subject to semver
#[expect(clippy::needless_pass_by_value, reason = "API simplicity")]
#[doc(hidden)]
pub fn ctid<T>(table: T) -> Ctid<T> {
    let _ = table;
    Ctid { table: PhantomData }
}

// EXPERIMENTAL: not subject to semver
#[derive(Debug, Copy, Clone, Default)]
#[doc(hidden)]
pub struct Ctid<T> {
    table: PhantomData<T>,
}

impl<T: Table> Ctid<T> {
    const SQFI: StaticQueryFragmentInstance<T> = StaticQueryFragmentInstance::<T>::new();
}

impl<T: Table> Expression for Ctid<T> {
    type SqlType = Tid;
}

impl<T: Table + 'static> QueryId for Ctid<T> {
    type QueryId = Self;
    const HAS_STATIC_QUERY_ID: bool = true;
}

impl<T: Table> Column for Ctid<T> {
    type Table = T;

    const NAME: &'static str = "ctid";
}

impl<T: Table, QS> SelectableExpression<QS> for Ctid<T> where
    QS: AppearsInFromClause<T, Count = Once>
{
}

impl<T: Table, QS> AppearsOnTable<QS> for Ctid<T> where QS: AppearsInFromClause<T, Count = Once> {}

impl<T: Table> ValidGrouping<()> for Ctid<T> {
    type IsAggregate = No;
}

impl<T: Table + StaticQueryFragment> QueryFragment<Pg> for Ctid<T>
where
    <T as StaticQueryFragment>::Component: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        if !pass.should_skip_from() {
            Self::SQFI.walk_ast(pass.reborrow())?;
            pass.push_sql(".");
        }
        pass.push_identifier(<Self as Column>::NAME)
    }
}
