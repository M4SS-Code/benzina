#![cfg(feature = "postgres")]

use benzina::{U15, extract_month, extract_year};
use diesel::{QueryDsl, debug_query, pg::Pg};

diesel::table! {
    events (id) {
        id -> Integer,
        d -> Date,
        ts -> Timestamp,
        nd -> Nullable<Date>,
    }
}

const SELECT: &str =
    r#"SELECT "events"."id", "events"."d", "events"."ts", "events"."nd" FROM "events" WHERE "#;

#[test]
fn year_eq_rewrites_to_make_date_range() {
    let q = events::table.filter(extract_year(events::d).eq(2024u16));
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!(
            "{SELECT}(\"events\".\"d\" BETWEEN make_date($1, 1, 1) AND make_date($2, 12, 31)) -- binds: [2024, 2024]"
        )
    );
}

#[test]
fn year_between_binds_both_years() {
    let q = events::table
        .filter(extract_year(events::d).between(U15::new(2020).unwrap(), U15::new(2024).unwrap()));
    assert!(
        debug_query::<Pg, _>(&q)
            .to_string()
            .ends_with("-- binds: [2020, 2024]")
    );
}

#[test]
fn year_range_works_on_nullable_date_columns() {
    let q = events::table.filter(extract_year(events::nd).eq(2024u16));
    assert!(
        debug_query::<Pg, _>(&q)
            .to_string()
            .ends_with("-- binds: [2024, 2024]")
    );
}

#[test]
fn i32_year_falls_back_to_date_part() {
    let q = events::table.filter(extract_year(events::d).eq(-5));
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!(
            "{SELECT}(CAST(date_part('year', \"events\".\"d\") AS integer) = $1) -- binds: [-5]"
        )
    );
}

#[test]
fn i32_year_works_on_timestamp_columns() {
    let q = events::table.filter(extract_year(events::ts).eq(2024));
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!(
            "{SELECT}(CAST(date_part('year', \"events\".\"ts\") AS integer) = $1) -- binds: [2024]"
        )
    );
}

#[test]
fn month_compares_the_indexable_expression() {
    let q = events::table.filter(extract_month(events::ts).between(3, 5));
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!(
            "{SELECT}(CAST(date_part('month', \"events\".\"ts\") AS integer) BETWEEN $1 AND $2) -- binds: [3, 5]"
        )
    );
}

#[test]
fn month_eq_month_compares_both_expressions() {
    let q = events::table
        .filter(extract_month(events::d).eq(extract_month(events::ts)))
        .select(events::id);
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        "SELECT \"events\".\"id\" FROM \"events\" WHERE (CAST(date_part('month', \"events\".\"d\") AS integer) = CAST(date_part('month', \"events\".\"ts\") AS integer)) -- binds: []"
    );
}
