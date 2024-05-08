use self::model::RuneBalanceEntity;
use self::model::RuneEventEntity;
use bitcoin::string;
use diesel::prelude::*;
use diesel::MysqlConnection;

use super::*;

mod runes_balance;
mod runes_entry;
mod runes_event;

pub struct RuneMysqlDao {}

pub fn new_db_conn(database_url: &str) -> MysqlConnection {
    MysqlConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub trait RuneEntryDao {
    fn gets_rune_entry(
        conn: &mut MysqlConnection,
        ids: Vec<String>,
    ) -> Result<Vec<RuneEntryEntity>>;
    fn load_entry_by_rune(conn: &mut MysqlConnection, _rune: &Rune) -> Result<RuneEntry>;
    fn load_rune_entry(conn: &mut MysqlConnection, id: &RuneId) -> Result<RuneEntry>;
    fn store_rune_entry(conn: &mut MysqlConnection, id: &RuneId, entry: &RuneEntry) -> Result<()>;
    fn update_rune_mints(conn: &mut MysqlConnection, id: &RuneId, _mints: u128) -> Result<()>;
    fn update_rune_burned(conn: &mut MysqlConnection, id: &RuneId, _burned: u128) -> Result<()>;
    fn delete_rune_entry(conn: &mut MysqlConnection, id: &RuneId) -> Result<()>;
    fn gets_rune_number(conn: &mut MysqlConnection) -> Option<u64>;
}

pub trait RuneEventDao {
    fn load(conn: &mut MysqlConnection, id: u64) -> Result<RuneEventEntity>;
    fn store_events(conn: &mut MysqlConnection, entry: &Vec<RuneEventEntity>) -> Result<()>;
    fn delete(conn: &mut MysqlConnection, id: &Txid) -> Result<()>;
}

pub trait RuneBlanaceDao {
    fn load_by_outpoint(
        conn: &mut MysqlConnection,
        outpoint: &OutPoint,
    ) -> Result<Vec<RuneBalanceEntity>>;
    fn update_spend_out_point(conn: &mut MysqlConnection, outpoint: &OutPoint) -> Result<()>;
    fn store_balances(conn: &mut MysqlConnection, entry: &Vec<RuneBalanceEntity>) -> Result<()>;
}
