//! PostgreSQL date equality via index-friendly ranges.

use std::marker::PhantomData;

use diesel::expression::{AsExpression, Expression};
use diesel::sql_types::{Bool, Date, Nullable, SqlType, Timestamp};

/// SQL types holding a plain calendar date — `Date` and `Nullable<Date>`.
///
/// Gates rewrites that compare a column against a *closed* date range: on
/// a timestamp column the range end would be promoted to midnight and cut
/// off the rest of the last day.
pub trait DateOnly: SqlType {}
impl DateOnly for Date {}
impl<T: DateOnly> DateOnly for Nullable<T> {}

/// SQL types accepted by [`extract_date`]: `Date` and `Timestamp`, plus
/// their `Nullable` variants.
///
/// `Timestamptz` is excluded on purpose: which instants fall on a calendar
/// date depends on the session time zone, so no fixed range is correct.
pub trait DateLike: SqlType {
    /// Marker picking the SQL emitted by [`DateEq`]: [`ClosedRange`] for
    /// dates, [`HalfOpenRange`] for timestamps.
    type Range;
}
impl DateLike for Date {
    type Range = ClosedRange;
}
impl DateLike for Timestamp {
    type Range = HalfOpenRange;
}
impl<T: DateLike> DateLike for Nullable<T> {
    type Range = T::Range;
}

/// Marker for `Date` columns, compared with a closed range.
///
/// A `Date` value *is* the whole day, so [`DateEq`] emits
/// `(expr BETWEEN rhs AND rhs)` — equivalent to `= rhs`.
#[derive(Debug, Clone, Copy)]
pub struct ClosedRange;

/// Marker for `Timestamp` columns, compared with a half-open range.
///
/// A day of timestamps has no last instant, so [`DateEq`] emits
/// `(expr >= rhs AND expr < rhs + 1)` — covering the whole day without
/// touching midnight of the next.
#[derive(Debug, Clone, Copy)]
pub struct HalfOpenRange;

/// Compares a date/timestamp expression against a `Date` as a range.
///
/// The column stays bare on one side, so PostgreSQL can drive a B-tree
/// index scan on it:
///
/// - `Date` columns: `(expr BETWEEN date AND date)` — equivalent to
///   `= date`
/// - `Timestamp` columns: `(expr >= date AND expr < date + 1)` — every
///   instant of that day, half-open so nothing past midnight is lost
///
/// `Timestamptz` columns are rejected: which instants fall on a calendar
/// date depends on the session time zone.
///
/// # Examples
///
/// ```
/// use benzina::extract_date;
/// use diesel::{
///     QueryDsl, debug_query,
///     dsl::{date, now},
///     pg::Pg,
/// };
///
/// diesel::table! {
///     events (id) {
///         id -> Integer,
///         day -> Date,
///         created_at -> Timestamp,
///     }
/// }
///
/// // `Date` column: closed range, equivalent to `= today`
/// let q = events::table.filter(extract_date(events::day).eq(date(now)));
/// assert!(debug_query::<Pg, _>(&q).to_string().contains(
///     r#"("events"."day" BETWEEN date(CURRENT_TIMESTAMP) AND date(CURRENT_TIMESTAMP))"#
/// ));
///
/// // `Timestamp` column: half-open range covering the whole day
/// let q = events::table.filter(extract_date(events::created_at).eq(date(now)));
/// assert!(debug_query::<Pg, _>(&q).to_string().contains(
///     r#"("events"."created_at" >= date(CURRENT_TIMESTAMP) AND "events"."created_at" < date(CURRENT_TIMESTAMP) + 1)"#
/// ));
/// ```
pub fn extract_date<Expr>(expr: Expr) -> ExtractedDate<Expr>
where
    Expr: Expression,
    Expr::SqlType: DateLike,
{
    ExtractedDate { expr }
}

/// The return type of [`extract_date`].
///
/// Compare it with [`eq`](ExtractedDate::eq). It is not an expression on
/// its own: the date is never computed in SQL, only rewritten into a
/// range comparison on the column.
#[derive(Debug, Clone, Copy, diesel::query_builder::QueryId)]
pub struct ExtractedDate<Expr> {
    expr: Expr,
}

impl<Expr> ExtractedDate<Expr> {
    /// Builds the range comparison against `rhs`, see [`extract_date`] for
    /// the SQL emitted per column type.
    ///
    /// Both sides can be emitted twice, so keep them to columns and binds —
    /// a volatile expression would be evaluated twice.
    pub fn eq<Rhs>(
        self,
        rhs: Rhs,
    ) -> DateEq<Expr, Rhs::Expression, <Expr::SqlType as DateLike>::Range>
    where
        Expr: Expression,
        Expr::SqlType: DateLike,
        Rhs: AsExpression<Date>,
    {
        DateEq {
            expr: self.expr,
            rhs: rhs.as_expression(),
            range: PhantomData,
        }
    }

    /// Returns the inner date/timestamp expression.
    pub fn into_inner(self) -> Expr {
        self.expr
    }
}

/// The return type of [`ExtractedDate::eq`].
///
/// Emits `(expr BETWEEN rhs AND rhs)` or `(expr >= rhs AND expr < rhs + 1)`,
/// picked by the `Range` marker — see [`extract_date`].
#[derive(Debug, Clone, Copy)]
pub struct DateEq<Expr, Rhs, Range> {
    expr: Expr,
    rhs: Rhs,
    range: PhantomData<Range>,
}

impl<Expr, Rhs, Range> diesel::query_builder::QueryId for DateEq<Expr, Rhs, Range>
where
    Expr: diesel::query_builder::QueryId,
    Rhs: diesel::query_builder::QueryId,
    Range: 'static,
{
    type QueryId = DateEq<Expr::QueryId, Rhs::QueryId, Range>;

    const HAS_STATIC_QUERY_ID: bool = Expr::HAS_STATIC_QUERY_ID && Rhs::HAS_STATIC_QUERY_ID;
}

impl<Expr, Rhs, Range> Expression for DateEq<Expr, Rhs, Range>
where
    Expr: Expression,
    Rhs: Expression,
{
    type SqlType = Bool;
}

impl<Expr, Rhs, DB> diesel::query_builder::QueryFragment<DB> for DateEq<Expr, Rhs, ClosedRange>
where
    Expr: diesel::query_builder::QueryFragment<DB>,
    Rhs: diesel::query_builder::QueryFragment<DB>,
    DB: diesel::backend::Backend,
{
    fn walk_ast<'b>(
        &'b self,
        mut out: diesel::query_builder::AstPass<'_, 'b, DB>,
    ) -> diesel::result::QueryResult<()> {
        out.push_sql("(");
        self.expr.walk_ast(out.reborrow())?;
        out.push_sql(" BETWEEN ");
        self.rhs.walk_ast(out.reborrow())?;
        out.push_sql(" AND ");
        self.rhs.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

impl<Expr, Rhs, DB> diesel::query_builder::QueryFragment<DB> for DateEq<Expr, Rhs, HalfOpenRange>
where
    Expr: diesel::query_builder::QueryFragment<DB>,
    Rhs: diesel::query_builder::QueryFragment<DB>,
    DB: diesel::backend::Backend,
{
    fn walk_ast<'b>(
        &'b self,
        mut out: diesel::query_builder::AstPass<'_, 'b, DB>,
    ) -> diesel::result::QueryResult<()> {
        out.push_sql("(");
        self.expr.walk_ast(out.reborrow())?;
        out.push_sql(" >= ");
        self.rhs.walk_ast(out.reborrow())?;
        out.push_sql(" AND ");
        self.expr.walk_ast(out.reborrow())?;
        out.push_sql(" < ");
        self.rhs.walk_ast(out.reborrow())?;
        out.push_sql(" + 1)");
        Ok(())
    }
}

impl<Expr, Rhs, Range, GB> diesel::expression::ValidGrouping<GB> for DateEq<Expr, Rhs, Range> {
    type IsAggregate = diesel::expression::is_aggregate::Never;
}

impl<Expr, Rhs, Range, QS> diesel::expression::SelectableExpression<QS> for DateEq<Expr, Rhs, Range>
where
    Self: Expression,
    Expr: diesel::expression::SelectableExpression<QS>,
    Rhs: diesel::expression::SelectableExpression<QS>,
{
}

impl<Expr, Rhs, Range, QS> diesel::expression::AppearsOnTable<QS> for DateEq<Expr, Rhs, Range>
where
    Self: Expression,
    Expr: diesel::expression::AppearsOnTable<QS>,
    Rhs: diesel::expression::AppearsOnTable<QS>,
{
}
