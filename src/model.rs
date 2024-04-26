use bigdecimal::BigDecimal;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::rune_entry)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub(crate) struct RuneEntryEntity {
    pub id: u64,
    pub block: u64,
    pub burned: BigDecimal,
    pub divisibility: u8,
    pub etching: String,
    pub mints: BigDecimal,
    pub number: u64,
    pub premine: BigDecimal,
    pub spaced_rune: String,
    pub rune_id: String,
    pub timestamp: u64,
    pub symbol: String,
    pub turbo: bool,
    pub amount: Option<BigDecimal>,
    pub cap: Option<BigDecimal>,
    pub height_start: Option<u64>,
    pub height_end: Option<u64>,
    pub offset_start: Option<u64>,
    pub offset_end: Option<u64>,
}
