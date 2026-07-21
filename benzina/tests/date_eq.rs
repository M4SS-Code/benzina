#![cfg(feature = "postgres")]

use benzina::extract_date;
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
fn date_eq_emits_a_between_range() {
    let q = events::table.filter(extract_date(events::nd).eq(events::d));
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!(
            "{SELECT}(\"events\".\"nd\" BETWEEN \"events\".\"d\" AND \"events\".\"d\") -- binds: []"
        )
    );
}

#[test]
fn date_eq_on_timestamp_covers_the_whole_day() {
    let q = events::table.filter(extract_date(events::ts).eq(events::d));
    assert_eq!(
        debug_query::<Pg, _>(&q).to_string(),
        format!(
            "{SELECT}(\"events\".\"ts\" >= \"events\".\"d\" AND \"events\".\"ts\" < \"events\".\"d\" + 1) -- binds: []"
        )
    );
}
