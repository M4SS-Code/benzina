//! PostgreSQL time-of-day expressions for comparing the time part of
//! time/timestamp columns.

use std::marker::PhantomData;

use diesel::expression::{AsExpression, Expression};
use diesel::sql_types::{Bool, Nullable, SqlType, Time, Timestamp};

/// SQL types accepted by [`extract_time`]: `Time` and `Timestamp`, plus
/// their `Nullable` variants.
///
/// `Timestamptz` is excluded on purpose: its cast to `time` depends on the
/// session time zone (the cast is only `STABLE`), so PostgreSQL also
/// rejects an expression index built on it.
///
/// Nullable inputs are accepted for filtering (a `NULL` comparison
/// excludes the row), but [`ExtractedTime`] still claims a non-nullable
/// `Time`, so don't `SELECT` it off a nullable column.
pub trait TimeLike: SqlType {
    /// Marker picking the SQL emitted by [`ExtractedTime`]: [`BareTime`]
    /// for time columns, [`CastToTime`] for timestamps.
    type Cast;
}
impl TimeLike for Time {
    type Cast = BareTime;
}
impl TimeLike for Timestamp {
    type Cast = CastToTime;
}
impl<T: TimeLike> TimeLike for Nullable<T> {
    type Cast = T::Cast;
}

/// Marker for `Time` columns — the value already is the time-of-day.
///
/// The column is emitted bare, so comparisons are driven by a plain
/// B-tree index on it.
#[derive(Debug, Clone, Copy)]
pub struct BareTime;

/// Marker for `Timestamp` columns — the time-of-day must be computed.
///
/// The column is emitted as `CAST(expr AS time)`, so comparisons can only
/// use an expression index built on exactly that expression, see
/// [`extract_time`].
#[derive(Debug, Clone, Copy)]
pub struct CastToTime;

/// Extracts the time-of-day of a time/timestamp expression.
///
/// The result is compared with the regular diesel operators (`.eq()`,
/// `.between()`, ...), which only accept `Time` values on the other side:
///
/// - `Time` columns are emitted bare, so a plain B-tree index on the
///   column drives the comparison; their [`eq`](ExtractedTime::eq) emits
///   the `(expr BETWEEN rhs AND rhs)` range form, like
///   [`extract_date`](crate::extract_date);
/// - `Timestamp` columns are emitted as `CAST(expr AS time)` and compared
///   with the plain operators, which can only use an expression index
///   built on exactly that expression:
///
/// ```sql
/// CREATE INDEX ... ON tbl ((CAST(col AS time)));
/// ```
///
/// (`(col)::time` parses to the same expression and matches too.)
///
/// Beware of `between` bounds crossing midnight: `BETWEEN '23:00' AND
/// '01:00'` is the empty range. Split it into a `.ge(..)` OR `.lt(..)`
/// pair instead.
///
/// # Examples
///
/// ```
/// use benzina::extract_time;
/// use diesel::{ExpressionMethods, QueryDsl, debug_query, pg::Pg};
///
/// diesel::table! {
///     shifts (id) {
///         id -> Integer,
///         starts_at -> Time,
///         created_at -> Timestamp,
///     }
/// }
///
/// // `Time` column: stays bare, `eq` uses the range form
/// let q = shifts::table
///     .filter(extract_time(shifts::starts_at).eq(extract_time(shifts::created_at)));
/// assert!(debug_query::<Pg, _>(&q).to_string().contains(
///     r#"("shifts"."starts_at" BETWEEN CAST("shifts"."created_at" AS time) AND CAST("shifts"."created_at" AS time))"#
/// ));
///
/// // `Timestamp` column: cast to `time`, compared with the regular operators
/// let q = shifts::table
///     .filter(extract_time(shifts::created_at).gt(extract_time(shifts::starts_at)));
/// assert!(debug_query::<Pg, _>(&q).to_string().contains(
///     r#"CAST("shifts"."created_at" AS time) > "shifts"."starts_at""#
/// ));
/// ```
pub fn extract_time<Expr>(expr: Expr) -> ExtractedTime<Expr, <Expr::SqlType as TimeLike>::Cast>
where
    Expr: Expression,
    Expr::SqlType: TimeLike,
{
    ExtractedTime {
        expr,
        cast: PhantomData,
    }
}

/// The return type of [`extract_time`].
///
/// Compare it with the regular diesel operators (`.eq()`, `.between()`,
/// ...), or with its own [`eq`](ExtractedTime::eq) on `Time` columns.
#[derive(Debug, Clone, Copy)]
pub struct ExtractedTime<Expr, Cast> {
    expr: Expr,
    cast: PhantomData<Cast>,
}

impl<Expr, Cast> ExtractedTime<Expr, Cast> {
    /// Returns the inner time/timestamp expression.
    pub fn into_inner(self) -> Expr {
        self.expr
    }
}

impl<Expr> ExtractedTime<Expr, BareTime> {
    /// `(expr BETWEEN rhs AND rhs)` — the same `Time` value on both sides,
    /// equivalent to `= rhs` and driven by a B-tree index on the column.
    ///
    /// Shadows [`ExpressionMethods::eq`](diesel::ExpressionMethods::eq) so
    /// that time columns get the same range form as
    /// [`extract_date`](crate::extract_date).
    ///
    /// `rhs` is emitted twice, so keep it to columns and binds — a
    /// volatile expression would be evaluated twice.
    pub fn eq<Rhs>(self, rhs: Rhs) -> TimeEq<Expr, Rhs::Expression>
    where
        Rhs: AsExpression<Time>,
    {
        TimeEq {
            expr: self.expr,
            rhs: rhs.as_expression(),
        }
    }
}

impl<Expr, Cast> diesel::query_builder::QueryId for ExtractedTime<Expr, Cast>
where
    Expr: diesel::query_builder::QueryId,
    Cast: 'static,
{
    type QueryId = ExtractedTime<Expr::QueryId, Cast>;

    const HAS_STATIC_QUERY_ID: bool = Expr::HAS_STATIC_QUERY_ID;
}

impl<Expr, Cast> Expression for ExtractedTime<Expr, Cast>
where
    Expr: Expression,
{
    type SqlType = Time;
}

impl<Expr, DB> diesel::query_builder::QueryFragment<DB> for ExtractedTime<Expr, BareTime>
where
    Expr: diesel::query_builder::QueryFragment<DB>,
    DB: diesel::backend::Backend,
{
    fn walk_ast<'b>(
        &'b self,
        out: diesel::query_builder::AstPass<'_, 'b, DB>,
    ) -> diesel::result::QueryResult<()> {
        self.expr.walk_ast(out)
    }
}

impl<Expr, DB> diesel::query_builder::QueryFragment<DB> for ExtractedTime<Expr, CastToTime>
where
    Expr: diesel::query_builder::QueryFragment<DB>,
    DB: diesel::backend::Backend,
{
    fn walk_ast<'b>(
        &'b self,
        mut out: diesel::query_builder::AstPass<'_, 'b, DB>,
    ) -> diesel::result::QueryResult<()> {
        out.push_sql("CAST(");
        self.expr.walk_ast(out.reborrow())?;
        out.push_sql(" AS time)");
        Ok(())
    }
}

// `ValidGrouping` is `Never` on purpose: Diesel can't verify computed
// `GROUP BY` expressions, so opt out of its aggregate checking (like
// `diesel::dsl::sql` does).
impl<Expr, Cast, GB> diesel::expression::ValidGrouping<GB> for ExtractedTime<Expr, Cast> {
    type IsAggregate = diesel::expression::is_aggregate::Never;
}

impl<Expr, Cast, QS> diesel::expression::SelectableExpression<QS> for ExtractedTime<Expr, Cast>
where
    Self: Expression,
    Expr: diesel::expression::SelectableExpression<QS>,
{
}

impl<Expr, Cast, QS> diesel::expression::AppearsOnTable<QS> for ExtractedTime<Expr, Cast>
where
    Self: Expression,
    Expr: diesel::expression::AppearsOnTable<QS>,
{
}

/// The return type of [`ExtractedTime::eq`].
///
/// Emits `(expr BETWEEN rhs AND rhs)`.
#[derive(Debug, Clone, Copy, diesel::query_builder::QueryId)]
pub struct TimeEq<Expr, Rhs> {
    expr: Expr,
    rhs: Rhs,
}

impl<Expr, Rhs> Expression for TimeEq<Expr, Rhs>
where
    Expr: Expression,
    Rhs: Expression,
{
    type SqlType = Bool;
}

impl<Expr, Rhs, DB> diesel::query_builder::QueryFragment<DB> for TimeEq<Expr, Rhs>
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

impl<Expr, Rhs, GB> diesel::expression::ValidGrouping<GB> for TimeEq<Expr, Rhs> {
    type IsAggregate = diesel::expression::is_aggregate::Never;
}

impl<Expr, Rhs, QS> diesel::expression::SelectableExpression<QS> for TimeEq<Expr, Rhs>
where
    Self: Expression,
    Expr: diesel::expression::SelectableExpression<QS>,
    Rhs: diesel::expression::SelectableExpression<QS>,
{
}

impl<Expr, Rhs, QS> diesel::expression::AppearsOnTable<QS> for TimeEq<Expr, Rhs>
where
    Self: Expression,
    Expr: diesel::expression::AppearsOnTable<QS>,
    Rhs: diesel::expression::AppearsOnTable<QS>,
{
}
