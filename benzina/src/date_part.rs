//! PostgreSQL `date_part()` expressions for extracting and comparing the
//! year, month and day of date/timestamp columns.

use diesel::{
    ExpressionMethods, dsl,
    expression::Expression,
    sql_types::{Date, Nullable, SqlType, Timestamp},
};

use crate::U15;
use crate::date_eq::DateOnly;

/// Date/timestamp SQL types accepted by [`extract_year`], [`extract_month`]
/// and [`extract_day`].
///
/// Nullable inputs are accepted for filtering (a `NULL` comparison excludes
/// the row), but the extract expressions still claim a non-nullable
/// `Integer`, so don't `SELECT` them off a nullable column.
pub trait DateOrTimestamp: SqlType {}
impl DateOrTimestamp for Date {}
impl DateOrTimestamp for Timestamp {}
impl<T: DateOrTimestamp> DateOrTimestamp for Nullable<T> {}

/// Right-hand side of the `eq` on [`YearPart`], [`MonthPart`] and
/// [`DayPart`].
///
/// Each impl decides the SQL emitted for its value type.
pub trait DatePartEq<Lhs> {
    /// The diesel expression type produced by the comparison.
    type Output;

    /// Builds the comparison expression with `lhs` on the left.
    fn eq_date_part(self, lhs: Lhs) -> Self::Output;
}

/// Bounds of the `between` on [`YearPart`], [`MonthPart`] and [`DayPart`].
///
/// Each impl decides the SQL emitted for its value type.
pub trait DatePartBetween<Lhs>: Sized {
    /// The diesel expression type produced by the comparison.
    type Output;

    /// Builds the inclusive range expression with `lhs` on the left.
    fn between_date_part(lhs: Lhs, lower: Self, upper: Self) -> Self::Output;
}

// `ValidGrouping` is `Never` on purpose: Diesel can't verify computed
// `GROUP BY` expressions, so opt out of its aggregate checking (like
// `diesel::dsl::sql` does).
macro_rules! impl_expression_boilerplate {
    ($ty:ident) => {
        impl<Expr, GB> diesel::expression::ValidGrouping<GB> for $ty<Expr> {
            type IsAggregate = diesel::expression::is_aggregate::Never;
        }

        impl<Expr, QS> diesel::expression::SelectableExpression<QS> for $ty<Expr>
        where
            Self: Expression,
            Expr: diesel::expression::SelectableExpression<QS>,
        {
        }

        impl<Expr, QS> diesel::expression::AppearsOnTable<QS> for $ty<Expr>
        where
            Self: Expression,
            Expr: diesel::expression::AppearsOnTable<QS>,
        {
        }
    };
}

// `CAST(date_part('<field>', expr) AS integer)` with the field baked into the
// type: a bind would break `SELECT`/`GROUP BY` matching (PostgreSQL compares
// the expressions structurally, and `$1` != `$4`), and a runtime field would
// break the type-keyed prepared-statement cache.
macro_rules! date_part_expr {
    ($ty:ident, $constructor:ident, $field:literal $(, $(#[$extra_doc:meta])+)? $(,)?) => {
        #[doc = concat!("The return type of [`", stringify!($constructor), "`].")]
        ///
        #[doc = concat!("Emits `CAST(date_part('", $field, "', expr) AS integer)`.")]
        #[derive(
            Debug, Clone, Copy, diesel::query_builder::QueryId, diesel::sql_types::DieselNumericOps,
        )]
        pub struct $ty<Expr> {
            expr: Expr,
        }

        #[doc = concat!("Extracts the ", $field, " of a date/timestamp expression as an integer.")]
        ///
        #[doc = concat!("Emits `CAST(date_part('", $field, "', expr) AS integer)`. Comparing")]
        /// this expression can only use an expression index built on exactly
        /// the emitted expression:
        ///
        /// ```sql
        #[doc = concat!("CREATE INDEX ... ON tbl ((CAST(date_part('", $field, "', col) AS integer)));")]
        /// ```
        ///
        /// An index built with `EXTRACT(..)` instead will *not* match:
        /// `EXTRACT` maps to a different catalog function than `date_part()`
        /// (they diverged in PostgreSQL 14; before that, `EXTRACT` was
        /// rewritten to `date_part()`).
        ///
        /// # Examples
        ///
        /// ```
        #[doc = concat!("use benzina::", stringify!($constructor), ";")]
        /// use diesel::{QueryDsl, debug_query, pg::Pg};
        ///
        /// diesel::table! {
        ///     events (id) {
        ///         id -> Integer,
        ///         created_at -> Timestamp,
        ///     }
        /// }
        ///
        #[doc = concat!("let q = events::table.filter(", stringify!($constructor), "(events::created_at).eq(7));")]
        /// assert!(debug_query::<Pg, _>(&q).to_string().contains(
        #[doc = concat!("    r#\"(CAST(date_part('", $field, "', \"events\".\"created_at\") AS integer) = $1)\"#")]
        /// ));
        /// ```
        $($(#[$extra_doc])+)?
        pub fn $constructor<Expr>(expr: Expr) -> $ty<Expr>
        where
            Expr: Expression,
            Expr::SqlType: DateOrTimestamp,
        {
            $ty { expr }
        }

        impl<Expr> Expression for $ty<Expr>
        where
            Expr: Expression,
        {
            type SqlType = diesel::sql_types::Integer;
        }

        impl<Expr, DB> diesel::query_builder::QueryFragment<DB> for $ty<Expr>
        where
            Expr: diesel::query_builder::QueryFragment<DB>,
            DB: diesel::backend::Backend,
        {
            fn walk_ast<'b>(
                &'b self,
                mut out: diesel::query_builder::AstPass<'_, 'b, DB>,
            ) -> diesel::result::QueryResult<()> {
                out.push_sql(concat!("CAST(date_part('", $field, "', "));
                self.expr.walk_ast(out.reborrow())?;
                out.push_sql(") AS integer)");
                Ok(())
            }
        }

        impl_expression_boilerplate!($ty);

        impl<Expr> $ty<Expr> {
            #[doc = concat!("Compares `date_part('", $field, "', expr)`; the generated SQL")]
            /// depends on the right-hand side, see [`DatePartEq`].
            ///
            /// Shadows [`ExpressionMethods::eq`] so that the right-hand side
            /// can pick a smarter SQL form than `date_part(..) = $1` where
            /// one exists.
            pub fn eq<Rhs>(self, rhs: Rhs) -> Rhs::Output
            where
                Rhs: DatePartEq<Self>,
            {
                rhs.eq_date_part(self)
            }

            #[doc = concat!("Range-compares `date_part('", $field, "', expr)`; the generated SQL")]
            /// depends on the bound type, see [`DatePartBetween`].
            ///
            /// Shadows [`ExpressionMethods::between`] so that the bounds can
            /// pick a smarter SQL form than `date_part(..) BETWEEN $1 AND $2`
            /// where one exists.
            pub fn between<Rhs>(self, lower: Rhs, upper: Rhs) -> Rhs::Output
            where
                Rhs: DatePartBetween<Self>,
            {
                Rhs::between_date_part(self, lower, upper)
            }

            /// Returns the inner date/timestamp expression.
            ///
            /// Useful for [`DatePartEq`] impls in downstream crates that
            /// rewrite the comparison into a range filter on the column
            /// itself.
            pub fn into_inner(self) -> Expr {
                self.expr
            }
        }

        #[doc = concat!("`date_part('", $field, "', a) = date_part('", $field, "', b)`")]
        impl<Expr, Expr2> DatePartEq<$ty<Expr>> for $ty<Expr2>
        where
            Expr: Expression,
            Expr2: Expression,
        {
            type Output = dsl::Eq<$ty<Expr>, $ty<Expr2>>;

            fn eq_date_part(self, lhs: $ty<Expr>) -> Self::Output {
                ExpressionMethods::eq(lhs, self)
            }
        }

        impl<Expr> DatePartEq<$ty<Expr>> for i32
        where
            Expr: Expression,
        {
            type Output = dsl::Eq<$ty<Expr>, i32>;

            fn eq_date_part(self, lhs: $ty<Expr>) -> Self::Output {
                ExpressionMethods::eq(lhs, self)
            }
        }

        impl<Expr> DatePartBetween<$ty<Expr>> for i32
        where
            Expr: Expression,
        {
            type Output = dsl::Between<$ty<Expr>, i32, i32>;

            fn between_date_part(lhs: $ty<Expr>, lower: Self, upper: Self) -> Self::Output {
                ExpressionMethods::between(lhs, lower, upper)
            }
        }
    };
}

date_part_expr!(
    YearPart,
    extract_year,
    "year",
    /// On date columns, bounded year types (`u8`, `u16`, [`U15`]) rewrite
    /// the comparison to a plain date range that can use a B-tree index on
    /// the column — see [`YearRange`]:
    ///
    /// ```
    /// use benzina::extract_year;
    /// use diesel::{QueryDsl, debug_query, pg::Pg};
    ///
    /// diesel::table! {
    ///     events (id) {
    ///         id -> Integer,
    ///         day -> Date,
    ///     }
    /// }
    ///
    /// let q = events::table.filter(extract_year(events::day).eq(2024u16));
    /// assert!(debug_query::<Pg, _>(&q).to_string().contains(
    ///     r#"("events"."day" BETWEEN make_date($1, 1, 1) AND make_date($2, 12, 31))"#
    /// ));
    /// ```
);
date_part_expr!(MonthPart, extract_month, "month");
date_part_expr!(DayPart, extract_day, "day");

/// The return type of [`YearPart::eq`] and [`YearPart::between`] on date
/// columns, for bounded year types.
///
/// "The year of `expr` is between `$1` and `$2`" is the same condition as
/// "`expr` is between Jan 1 of `$1` and Dec 31 of `$2`", so the comparison
/// is rewritten to
/// `(expr BETWEEN make_date($1, 1, 1) AND make_date($2, 12, 31))`: the
/// column stays bare, letting PostgreSQL use a plain B-tree index on it
/// instead of computing `date_part('year', expr)` for every row.
///
/// The rewrite only happens for year values that `make_date()` always
/// accepts — `u8`, `u16` and [`U15`]; see their [`DatePartEq`] and
/// [`DatePartBetween`] impls for the year-0 runtime caveat.
#[derive(Debug, Clone, Copy, diesel::query_builder::QueryId)]
pub struct YearRange<Expr> {
    expr: Expr,
    from_year: i32,
    to_year: i32,
}

impl<Expr> Expression for YearRange<Expr>
where
    Expr: Expression,
{
    type SqlType = diesel::sql_types::Bool;
}

impl<Expr, DB> diesel::query_builder::QueryFragment<DB> for YearRange<Expr>
where
    Expr: diesel::query_builder::QueryFragment<DB>,
    DB: diesel::backend::Backend,
    i32: diesel::serialize::ToSql<diesel::sql_types::Integer, DB>,
{
    fn walk_ast<'b>(
        &'b self,
        mut out: diesel::query_builder::AstPass<'_, 'b, DB>,
    ) -> diesel::result::QueryResult<()> {
        out.push_sql("(");
        self.expr.walk_ast(out.reborrow())?;
        out.push_sql(" BETWEEN make_date(");
        out.push_bind_param::<diesel::sql_types::Integer, _>(&self.from_year)?;
        out.push_sql(", 1, 1) AND make_date(");
        out.push_bind_param::<diesel::sql_types::Integer, _>(&self.to_year)?;
        out.push_sql(", 12, 31))");
        Ok(())
    }
}

impl_expression_boilerplate!(YearRange);

// Year types that can't name BC years, whose values are all valid
// `make_date()` years except 0 (documented on the impls). `i32`
// deliberately gets only the plain `date_part(..)` comparison instead: part
// of its range are years that PostgreSQL rejects at runtime.
macro_rules! year_range_bounds {
    ($($ty:ty),*) => {$(
        /// `(expr BETWEEN make_date(year, 1, 1) AND make_date(year, 12, 31))`
        ///
        /// Equivalent to comparing `date_part('year', expr)` on a date column,
        /// but the plain range comparison lets PostgreSQL use a B-tree index
        /// on the column. Only the `i32` year is bound, so this works no
        /// matter which Rust date library the application uses.
        ///
        /// PostgreSQL has no year 0: passing 0 fails at runtime with
        /// `date field value out of range`.
        impl<Expr> DatePartEq<YearPart<Expr>> for $ty
        where
            Expr: Expression,
            Expr::SqlType: DateOnly,
        {
            type Output = YearRange<Expr>;

            fn eq_date_part(self, lhs: YearPart<Expr>) -> Self::Output {
                DatePartBetween::between_date_part(lhs, self, self)
            }
        }

        /// `(expr BETWEEN make_date(lower, 1, 1) AND make_date(upper, 12, 31))`
        ///
        /// Equivalent to range-comparing `date_part('year', expr)` on a date
        /// column (a span of whole years is still a single date range), but
        /// lets PostgreSQL use a B-tree index on the column. Only the `i32`
        /// years are bound, so this works no matter which Rust date library
        /// the application uses.
        ///
        /// PostgreSQL has no year 0: passing 0 as either bound fails at
        /// runtime with `date field value out of range`.
        impl<Expr> DatePartBetween<YearPart<Expr>> for $ty
        where
            Expr: Expression,
            Expr::SqlType: DateOnly,
        {
            type Output = YearRange<Expr>;

            fn between_date_part(lhs: YearPart<Expr>, lower: Self, upper: Self) -> Self::Output {
                YearRange {
                    expr: lhs.expr,
                    from_year: i32::from(lower),
                    to_year: i32::from(upper),
                }
            }
        }
    )*};
}

year_range_bounds!(u8, u16, U15);
