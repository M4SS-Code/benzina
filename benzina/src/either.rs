use diesel::backend::Backend;
use diesel::expression::ValidGrouping;
use diesel::query_builder::{AstPass, QueryFragment, QueryId};
use diesel::{AppearsOnTable, Expression, QueryResult, SelectableExpression};

/// Either type for Diesel expressions - allows different expression types in match arms.
#[derive(Debug, Clone, Copy)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Expression for Either<L, R>
where
    L: Expression,
    R: Expression<SqlType = L::SqlType>,
{
    type SqlType = L::SqlType;
}

impl<L, R, QS> AppearsOnTable<QS> for Either<L, R>
where
    Self: Expression,
    L: AppearsOnTable<QS>,
    R: AppearsOnTable<QS>,
{
}

impl<L, R, GB> ValidGrouping<GB> for Either<L, R>
where
    L: ValidGrouping<GB>,
    R: ValidGrouping<GB, IsAggregate = L::IsAggregate>,
{
    type IsAggregate = L::IsAggregate;
}

impl<L, R, QS> SelectableExpression<QS> for Either<L, R>
where
    Self: AppearsOnTable<QS>,
    L: SelectableExpression<QS>,
    R: SelectableExpression<QS>,
{
}

impl<L, R, DB> QueryFragment<DB> for Either<L, R>
where
    DB: Backend,
    L: QueryFragment<DB>,
    R: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        match self {
            Either::Left(l) => l.walk_ast(pass),
            Either::Right(r) => r.walk_ast(pass),
        }
    }
}

impl<L, R> QueryId for Either<L, R> {
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}
