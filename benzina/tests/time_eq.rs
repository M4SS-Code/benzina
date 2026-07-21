#![cfg(feature = "postgres")]

use benzina::extract_time;
use diesel::{ExpressionMethods, QueryDsl, debug_query, pg::Pg};

diesel::table! {
    events (id) {
        id -> Integer,
        t -> Time,
        ts -> Timestamp,
        nt -> Nullable<Time>,
    }
}

const SELECT: &str =
    r#"SELECT "events"."id", "events"."t", "events"."ts", "events"."nt" FROM "events" WHERE "#;

#[test]
fn time_column_eq_emits_a_between_range() {
    let q = events::table.filter(extract_time(events::nt).eq(events::t));
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!(
            "{SELECT}(\"events\".\"nt\" BETWEEN \"events\".\"t\" AND \"events\".\"t\") -- binds: []"
        )
    );
}

#[test]
fn timestamp_column_is_cast_to_time() {
    let q = events::table.filter(extract_time(events::ts).eq(events::t));
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!("{SELECT}(CAST(\"events\".\"ts\" AS time) = \"events\".\"t\") -- binds: []")
    );
}

#[test]
fn between_compares_the_cast_expression() {
    let q = events::table.filter(extract_time(events::ts).between(events::t, events::t));
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!(
            "{SELECT}(CAST(\"events\".\"ts\" AS time) BETWEEN \"events\".\"t\" AND \"events\".\"t\") -- binds: []"
        )
    );
}

#[test]
fn time_eq_time_ranges_over_the_cast_expression() {
    let q = events::table
        .filter(extract_time(events::t).eq(extract_time(events::ts)))
        .select(events::id);
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        "SELECT \"events\".\"id\" FROM \"events\" WHERE (\"events\".\"t\" BETWEEN CAST(\"events\".\"ts\" AS time) AND CAST(\"events\".\"ts\" AS time)) -- binds: []"
    );
}
