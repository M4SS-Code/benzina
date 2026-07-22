//! PostgreSQL `AT TIME ZONE` conversions with user-declared time zones,
//! bridging `Timestamptz` columns to the `extract_*` expressions.

use std::marker::PhantomData;

use diesel::expression::Expression;
use diesel::sql_types::{Nullable, SingleValue, SqlType, Timestamp, Timestamptz};

/// A PostgreSQL time zone, as a compile-time marker type.
///
/// Usually declared with the [`timezone!`](crate::timezone) macro and
/// applied with [`at`](Self::at):
///
/// ```
/// use benzina::{TimeZone, extract_date};
/// use diesel::{
///     ExpressionMethods, QueryDsl, debug_query,
///     dsl::{date, now},
///     pg::Pg,
/// };
///
/// benzina::timezone!(pub Rome = "Europe/Rome");
///
/// diesel::table! {
///     events (id) {
///         id -> Integer,
///         created_at -> Timestamptz,
///     }
/// }
///
/// // Rows created today on the Rome calendar
/// let q = events::table.filter(extract_date(Rome::at(events::created_at)).eq(date(now)));
/// assert!(debug_query::<Pg, _>(&q).to_string().contains(
///     r#"(("events"."created_at") AT TIME ZONE 'Europe/Rome') >= date(CURRENT_TIMESTAMP)"#
/// ));
/// ```
///
/// # SQL literal
///
/// [`NAME`](Self::NAME) is spliced into the SQL as a literal, not a bind: a
/// bind would break `SELECT`/`GROUP BY` matching (PostgreSQL compares the
/// expressions structurally, and `$1` != `$4`), and a runtime name would
/// break the type-keyed prepared-statement cache.
pub trait TimeZone {
    /// The PostgreSQL time zone name, e.g. `Europe/Rome`.
    ///
    /// Spliced verbatim between the quotes of a SQL literal, so — like
    /// [`diesel::dsl::sql`] — it must be valid SQL there. Whether it names
    /// an *existing* zone is only checked by PostgreSQL at query time
    /// (`time zone "..." not recognized`). Prefer IANA names
    /// (`Europe/Rome`): POSIX-style offsets (`UTC+2`) are also accepted by
    /// PostgreSQL but with the sign meaning inverted from ISO 8601.
    const NAME: &'static str;

    /// Converts a timestamp expression to this zone's wall-clock time, see
    /// [`at_time_zone`].
    ///
    /// # Examples
    ///
    /// ```
    /// use benzina::{TimeZone, extract_year};
    /// use diesel::{QueryDsl, debug_query, pg::Pg};
    ///
    /// benzina::timezone!(pub Rome = "Europe/Rome");
    ///
    /// diesel::table! {
    ///     events (id) {
    ///         id -> Integer,
    ///         created_at -> Timestamptz,
    ///     }
    /// }
    ///
    /// let q = events::table.filter(extract_year(Rome::at(events::created_at)).eq(2024));
    /// assert!(debug_query::<Pg, _>(&q).to_string().contains(
    ///     r#"(CAST(date_part('year', (("events"."created_at") AT TIME ZONE 'Europe/Rome')) AS integer) = $1)"#
    /// ));
    /// ```
    fn at<Expr>(expr: Expr) -> AtTimeZone<Expr, Self, <Expr::SqlType as InstantLike>::Conversion>
    where
        Self: Sized,
        Expr: Expression,
        Expr::SqlType: InstantLike,
    {
        at_time_zone::<Self, Expr>(expr)
    }
}

/// Timestamp SQL types accepted by [`at_time_zone`]: `Timestamptz` and
/// `Timestamp`, plus their `Nullable` variants.
///
/// Nullability propagates: converting a `Nullable<Timestamptz>` expression
/// yields a `Nullable<Timestamp>` one.
pub trait InstantLike: SqlType {
    /// Marker picking the SQL emitted by [`AtTimeZone`]: [`FromInstant`]
    /// for `Timestamptz`, [`FromUtc`] for `Timestamp`.
    type Conversion;

    /// The SQL type of the converted expression — the zone's wall-clock
    /// `Timestamp`, `Nullable` when the input is.
    type Output: SqlType + SingleValue;
}
impl InstantLike for Timestamptz {
    type Conversion = FromInstant;
    type Output = Timestamp;
}
impl InstantLike for Timestamp {
    type Conversion = FromUtc;
    type Output = Timestamp;
}
impl<T: InstantLike> InstantLike for Nullable<T> {
    type Conversion = T::Conversion;
    type Output = Nullable<T::Output>;
}

/// Marker for `Timestamptz` columns — the value already is an instant.
///
/// The expression is emitted as `((expr) AT TIME ZONE 'NAME')`.
#[derive(Debug, Clone, Copy)]
pub struct FromInstant;

/// Marker for `Timestamp` columns **assumed to store UTC wall-clock time**
/// (the common convention for naive timestamps).
///
/// The expression is emitted as
/// `(((expr) AT TIME ZONE 'UTC') AT TIME ZONE 'NAME')`: the first
/// conversion reinterprets the naive value as a UTC instant, the second
/// renders it in the target zone.
#[derive(Debug, Clone, Copy)]
pub struct FromUtc;

/// Converts a timestamp expression to `Tz`'s wall-clock time.
///
/// The result is a plain (`Nullable<`)`Timestamp`(`>`) expression, so the
/// whole `extract_*` family composes with it —
/// [`extract_date`](crate::extract_date),
/// [`extract_time`](crate::extract_time),
/// [`extract_year`](crate::extract_year)/month/day — which is how those
/// are used with `Timestamptz` columns.
///
/// `timezone(zone, timestamp)` is `IMMUTABLE` in PostgreSQL (unlike the
/// session-dependent casts that keep `Timestamptz` out of
/// [`DateLike`](crate::DateLike) and [`TimeLike`](crate::TimeLike)), so
/// comparisons can use an expression index built on the emitted
/// expression:
///
/// ```sql
/// CREATE INDEX ... ON tbl ((col AT TIME ZONE 'Europe/Rome'));
/// ```
///
/// (PostgreSQL matches indexes on the parse tree, so parenthesization
/// doesn't matter.) Note the `IMMUTABLE` marking is a pragmatic lie: a
/// political change to the zone's rules can make already-indexed values
/// stale until the index is rebuilt.
///
/// # Examples
///
/// ```
/// use benzina::{at_time_zone, extract_date, extract_time};
/// use diesel::{
///     ExpressionMethods, QueryDsl, debug_query,
///     dsl::{date, now},
///     pg::Pg,
/// };
///
/// benzina::timezone!(pub Rome = "Europe/Rome");
///
/// diesel::table! {
///     events (id) {
///         id -> Integer,
///         created_at -> Timestamptz,
///         recorded_at -> Timestamp,
///         starts_at -> Time,
///     }
/// }
///
/// // `Timestamptz` column: single conversion
/// let q = events::table.select(at_time_zone::<Rome, _>(events::created_at));
/// assert!(debug_query::<Pg, _>(&q).to_string().contains(
///     r#"(("events"."created_at") AT TIME ZONE 'Europe/Rome')"#
/// ));
///
/// // `Timestamp` column storing UTC: reinterpreted as UTC first
/// let q = events::table.select(at_time_zone::<Rome, _>(events::recorded_at));
/// assert!(debug_query::<Pg, _>(&q).to_string().contains(
///     r#"((("events"."recorded_at") AT TIME ZONE 'UTC') AT TIME ZONE 'Europe/Rome')"#
/// ));
///
/// // Rows on today's Rome calendar day: `extract_date` compares the
/// // converted expression as a half-open range
/// let q = events::table
///     .filter(extract_date(at_time_zone::<Rome, _>(events::created_at)).eq(date(now)));
/// assert!(debug_query::<Pg, _>(&q).to_string().contains(
///     r#"((("events"."created_at") AT TIME ZONE 'Europe/Rome') >= date(CURRENT_TIMESTAMP) AND (("events"."created_at") AT TIME ZONE 'Europe/Rome') < date(CURRENT_TIMESTAMP) + 1)"#
/// ));
///
/// // Rows after a Rome time-of-day: `extract_time` casts the converted
/// // expression to `time`
/// let q = events::table
///     .filter(extract_time(at_time_zone::<Rome, _>(events::created_at)).gt(events::starts_at));
/// assert!(debug_query::<Pg, _>(&q).to_string().contains(
///     r#"CAST((("events"."created_at") AT TIME ZONE 'Europe/Rome') AS time) > "events"."starts_at""#
/// ));
/// ```
///
/// Only timestamp types are accepted:
///
/// ```compile_fail
/// use benzina::TimeZone;
///
/// benzina::timezone!(pub Rome = "Europe/Rome");
///
/// diesel::table! {
///     events (id) {
///         id -> Integer,
///         day -> Date,
///     }
/// }
///
/// let expr = Rome::at(events::day); // `Date` has no time zone to convert
/// ```
pub fn at_time_zone<Tz, Expr>(
    expr: Expr,
) -> AtTimeZone<Expr, Tz, <Expr::SqlType as InstantLike>::Conversion>
where
    Tz: TimeZone,
    Expr: Expression,
    Expr::SqlType: InstantLike,
{
    AtTimeZone {
        expr,
        marker: PhantomData,
    }
}

/// The return type of [`at_time_zone`] and [`TimeZone::at`].
#[derive(Debug, Clone, Copy, diesel::sql_types::DieselNumericOps)]
pub struct AtTimeZone<Expr, Tz, Conv> {
    expr: Expr,
    marker: PhantomData<(Tz, Conv)>,
}

impl<Expr, Tz, Conv> AtTimeZone<Expr, Tz, Conv> {
    /// Returns the inner timestamp expression.
    pub fn into_inner(self) -> Expr {
        self.expr
    }
}

impl<Expr, Tz, Conv> diesel::query_builder::QueryId for AtTimeZone<Expr, Tz, Conv>
where
    Expr: diesel::query_builder::QueryId,
    Tz: 'static,
    Conv: 'static,
{
    type QueryId = AtTimeZone<Expr::QueryId, Tz, Conv>;

    const HAS_STATIC_QUERY_ID: bool = Expr::HAS_STATIC_QUERY_ID;
}

impl<Expr, Tz, Conv> Expression for AtTimeZone<Expr, Tz, Conv>
where
    Expr: Expression,
    Expr::SqlType: InstantLike,
{
    type SqlType = <Expr::SqlType as InstantLike>::Output;
}

// The walked expression needs its own parens: Diesel emits arithmetic
// unparenthesized and `AT TIME ZONE` binds tighter than `+`.
impl<Expr, Tz, DB> diesel::query_builder::QueryFragment<DB> for AtTimeZone<Expr, Tz, FromInstant>
where
    Expr: diesel::query_builder::QueryFragment<DB>,
    Tz: TimeZone,
    DB: diesel::backend::Backend,
{
    fn walk_ast<'b>(
        &'b self,
        mut out: diesel::query_builder::AstPass<'_, 'b, DB>,
    ) -> diesel::result::QueryResult<()> {
        out.push_sql("((");
        self.expr.walk_ast(out.reborrow())?;
        out.push_sql(") AT TIME ZONE '");
        out.push_sql(Tz::NAME);
        out.push_sql("')");
        Ok(())
    }
}

impl<Expr, Tz, DB> diesel::query_builder::QueryFragment<DB> for AtTimeZone<Expr, Tz, FromUtc>
where
    Expr: diesel::query_builder::QueryFragment<DB>,
    Tz: TimeZone,
    DB: diesel::backend::Backend,
{
    fn walk_ast<'b>(
        &'b self,
        mut out: diesel::query_builder::AstPass<'_, 'b, DB>,
    ) -> diesel::result::QueryResult<()> {
        out.push_sql("(((");
        self.expr.walk_ast(out.reborrow())?;
        out.push_sql(") AT TIME ZONE 'UTC') AT TIME ZONE '");
        out.push_sql(Tz::NAME);
        out.push_sql("')");
        Ok(())
    }
}

// `ValidGrouping` is `Never` on purpose: Diesel can't verify computed
// `GROUP BY` expressions, so opt out of its aggregate checking (like
// `diesel::dsl::sql` does).
impl<Expr, Tz, Conv, GB> diesel::expression::ValidGrouping<GB> for AtTimeZone<Expr, Tz, Conv> {
    type IsAggregate = diesel::expression::is_aggregate::Never;
}

impl<Expr, Tz, Conv, QS> diesel::expression::SelectableExpression<QS> for AtTimeZone<Expr, Tz, Conv>
where
    Self: Expression,
    Expr: diesel::expression::SelectableExpression<QS>,
{
}

impl<Expr, Tz, Conv, QS> diesel::expression::AppearsOnTable<QS> for AtTimeZone<Expr, Tz, Conv>
where
    Self: Expression,
    Expr: diesel::expression::AppearsOnTable<QS>,
{
}

/// Declares one or more time zones for use with
/// [`at_time_zone`](crate::at_time_zone).
///
/// Each declaration expands to a unit struct implementing
/// [`TimeZone`](crate::TimeZone). The zone name is spliced verbatim into
/// the SQL literal, so it must be valid SQL there; whether the zone
/// *exists* is only checked by PostgreSQL at query time.
///
/// # Examples
///
/// ```
/// use benzina::TimeZone;
///
/// benzina::timezone!(
///     /// You can add documentation.
///     pub Rome = "Europe/Rome",
///     NewYork = "America/New_York",
/// );
///
/// assert_eq!(Rome::NAME, "Europe/Rome");
/// ```
#[macro_export]
macro_rules! timezone {
    (
        $(
            $(#[$attr:meta])*
            $vis:vis $name:ident = $tz:expr
        ),+ $(,)?
    ) => {
        $(
            $(#[$attr])*
            #[derive(
                $crate::__private::std::fmt::Debug,
                $crate::__private::std::clone::Clone,
                $crate::__private::std::marker::Copy,
            )]
            $vis struct $name;

            impl $crate::TimeZone for $name {
                const NAME: &'static str = $tz;
            }
        )+
    };
}
