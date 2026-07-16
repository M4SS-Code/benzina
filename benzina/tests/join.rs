use diesel::Identifiable;

diesel::table! {
    groups (id) {
        id -> Integer,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
    }
}

#[derive(Debug, PartialEq, Eq, Identifiable)]
#[diesel(table_name = groups)]
struct Group {
    id: i32,
}

#[derive(Debug, PartialEq, Eq, Identifiable)]
#[diesel(table_name = users)]
struct User {
    id: i32,
}

#[derive(Debug, PartialEq, Eq)]
struct GroupWithUsers {
    group: Group,
    users: Vec<User>,
}

#[test]
fn collects_direct_vec_values() {
    let records = vec![
        (Group { id: 1 }, User { id: 10 }),
        (Group { id: 1 }, User { id: 11 }),
        (Group { id: 1 }, User { id: 10 }),
        (Group { id: 2 }, User { id: 12 }),
    ];

    let joined = benzina::join! {
        records,
        Vec<GroupWithUsers {
            group: One<0>,
            users: Vec<1>,
        }>,
    };

    assert_eq!(
        vec![
            GroupWithUsers {
                group: Group { id: 1 },
                users: vec![User { id: 10 }, User { id: 11 }],
            },
            GroupWithUsers {
                group: Group { id: 2 },
                users: vec![User { id: 12 }],
            },
        ],
        joined,
    );
}
