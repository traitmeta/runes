use bigdecimal::BigDecimal;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::rune_entry)]
#[diesel(primary_key(id))]
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
    pub rune: BigDecimal,
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

#[derive(Queryable, Selectable, Insertable, Default)]
#[diesel(table_name = crate::schema::rune_event)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub(crate) struct RuneEventEntity {
    pub id: u64,
    pub block: u64,
    pub event_type: u8,
    pub tx_id: String,
    pub rune_id: String,
    pub amount: Option<BigDecimal>,
    pub address: String,
    pub pk_script_hex: String,
    pub vout: u32,
    pub rune_stone: String,
    pub timestamp: u64,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::rune_balance)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub(crate) struct RuneBalanceEntity {
    pub id: u64,
    pub block: u64,
    pub rune_id: String,
    pub amount: BigDecimal,
    pub address: String,
    pub pk_script_hex: String,
    pub out_point: String,
    pub spent: bool,
}
