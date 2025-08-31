use std::io::Write as _;

use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::{Pg, PgValue},
    query_builder::QueryId,
    serialize::{self, IsNull, Output, ToSql},
    sql_types::SqlType,
};

#[derive(Debug, Copy, Clone, Default, QueryId, SqlType)]
#[diesel(postgres_type(oid = 27, array_oid = 1010))]
pub struct Tid;

#[derive(Debug, Copy, Clone, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = Tid)]
pub struct TidValue {
    pub block_number: u32,
    pub offset_number: u16,
}

impl FromSql<Tid, Pg> for TidValue {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let buf = bytes.as_bytes();
        let (&block_number, buf) = buf.split_first_chunk::<4>().ok_or("invalid block number")?;
        let block_number = u32::from_be_bytes(block_number);
        let &offset_number = buf.first_chunk::<2>().ok_or("invalid offset number")?;
        let offset_number = u16::from_be_bytes(offset_number);
        Ok(Self {
            block_number,
            offset_number,
        })
    }
}

impl ToSql<Tid, Pg> for TidValue {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let mut buf = [0u8; 4 + 2];
        buf[..4].copy_from_slice(&self.block_number.to_be_bytes());
        buf[4..].copy_from_slice(&self.offset_number.to_be_bytes());
        out.write_all(&buf).map(|()| IsNull::No).map_err(Into::into)
    }
}
