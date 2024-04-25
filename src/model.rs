use diesel::prelude::*;
use bigdecimal::BigDecimal;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::rune_entry)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
struct RuneEntryEntity {
    pub id: u64,
    pub block: i64,
    pub burned: BigDecimal,
    pub divisibility: i32,
    pub etching: String,
    pub mints: BigDecimal,
    pub number: i64,
    pub premine: BigDecimal,
    pub spaced_rune: String,
    pub timestamp: i64,
    pub symbol: String,
    pub turbo: bool,
    pub amount: Option<BigDecimal>,
    pub cap: Option<BigDecimal>,
    pub height_start: Option<u64>,
    pub height_end: Option<u64>,
    pub offset_start: Option<u64>,
    pub offset_end: Option<u64>,
}