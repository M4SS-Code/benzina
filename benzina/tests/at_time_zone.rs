#![cfg(feature = "postgres")]

use benzina::{TimeZone, at_time_zone, extract_date, extract_time, extract_year};
use diesel::{
    ExpressionMethods, QueryDsl, debug_query,
    dsl::{date, now},
    pg::Pg,
};

benzina::timezone!(
    /// Rome wall clock.
    pub Rome = "Europe/Rome",
);

diesel::table! {
    events (id) {
        id -> Integer,
        created_at -> Timestamptz,
        recorded_at -> Timestamp,
        deleted_at -> Nullable<Timestamptz>,
        t -> Time,
    }
}

const ROME_CREATED_AT: &str = r#"(("events"."created_at") AT TIME ZONE 'Europe/Rome')"#;

#[test]
fn timestamptz_column_converts_with_a_single_hop() {
    let q = events::table.select(Rome::at(events::created_at));
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!("SELECT {ROME_CREATED_AT} FROM \"events\" -- binds: []")
    );
}

#[test]
fn timestamp_column_is_reinterpreted_as_utc_first() {
    let q = events::table.select(Rome::at(events::recorded_at));
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        "SELECT (((\"events\".\"recorded_at\") AT TIME ZONE 'UTC') AT TIME ZONE 'Europe/Rome') FROM \"events\" -- binds: []"
    );
}

#[test]
fn free_fn_matches_the_trait_method() {
    let q = events::table.select(at_time_zone::<Rome, _>(events::created_at));
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!("SELECT {ROME_CREATED_AT} FROM \"events\" -- binds: []")
    );
}

#[test]
fn nullable_input_yields_a_nullable_output() {
    fn assert_nullable_timestamp<E>(_: E)
    where
        E: diesel::expression::Expression<
                SqlType = diesel::sql_types::Nullable<diesel::sql_types::Timestamp>,
            >,
    {
    }
    assert_nullable_timestamp(Rome::at(events::deleted_at));

    let q = events::table.select(Rome::at(events::deleted_at));
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        "SELECT ((\"events\".\"deleted_at\") AT TIME ZONE 'Europe/Rome') FROM \"events\" -- binds: []"
    );
}

#[test]
fn extract_year_composes() {
    let q = events::table
        .filter(extract_year(Rome::at(events::created_at)).eq(2024))
        .select(events::id);
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!(
            "SELECT \"events\".\"id\" FROM \"events\" WHERE (CAST(date_part('year', {ROME_CREATED_AT}) AS integer) = $1) -- binds: [2024]"
        )
    );
}

#[test]
fn extract_date_uses_the_half_open_range() {
    let q = events::table
        .filter(extract_date(Rome::at(events::created_at)).eq(date(now)))
        .select(events::id);
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!(
            "SELECT \"events\".\"id\" FROM \"events\" WHERE ({ROME_CREATED_AT} >= date(CURRENT_TIMESTAMP) AND {ROME_CREATED_AT} < date(CURRENT_TIMESTAMP) + 1) -- binds: []"
        )
    );
}

#[test]
fn extract_time_casts_the_converted_expression() {
    let q = events::table
        .filter(extract_time(Rome::at(events::created_at)).gt(events::t))
        .select(events::id);
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!(
            "SELECT \"events\".\"id\" FROM \"events\" WHERE (CAST({ROME_CREATED_AT} AS time) > \"events\".\"t\") -- binds: []"
        )
    );
}

#[test]
fn select_and_group_by_match_structurally() {
    let q = events::table
        .group_by(extract_year(Rome::at(events::created_at)))
        .select(extract_year(Rome::at(events::created_at)));
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!(
            "SELECT CAST(date_part('year', {ROME_CREATED_AT}) AS integer) FROM \"events\" GROUP BY CAST(date_part('year', {ROME_CREATED_AT}) AS integer) -- binds: []"
        )
    );
}
