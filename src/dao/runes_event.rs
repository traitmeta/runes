use super::*;

impl RuneEventDao for RuneMysqlDao {
    fn load_by_outpoint(
        conn: &mut MysqlConnection,
        outpoint: &OutPoint,
    ) -> Result<Vec<RuneEventEntity>> {
        use self::schema::rune_event::{tx_id, vout};
        let results = RuneEventTable
            .filter(tx_id.eq(outpoint.txid.to_string()))
            .filter(vout.eq(outpoint.vout))
            .select(RuneEventEntity::as_select())
            .load(conn);

        match results {
            Ok(events) => Ok(events),
            Err(e) => Err(e.into()),
        }
    }

    fn load(conn: &mut MysqlConnection, _id: u64) -> Result<RuneEventEntity> {
        use self::schema::rune_event::id;
        let result = RuneEventTable
            .filter(id.eq(_id))
            .select(RuneEventEntity::as_select())
            .first(conn);

        match result {
            Ok(entity) => Ok(entity),
            Err(e) => Err(e.into()),
        }
    }

    fn store(conn: &mut MysqlConnection, entity: &RuneEventEntity) -> Result<()> {
        let insert_rows = diesel::insert_into(RuneEventTable)
            .values(entity)
            .execute(conn)
            .unwrap();

        if insert_rows == 0 {
            return Err(anyhow!("insert rune entry failed"));
        }

        Ok(())
    }

    fn delete(conn: &mut MysqlConnection, id: &Txid) -> Result<()> {
        use crate::schema::rune_event::tx_id;
        let effect_rows = diesel::delete(RuneEventTable.filter(tx_id.eq(id.to_string())))
            .execute(conn)
            .expect("Error deleting rune entry");

        if effect_rows == 0 {
            return Err(anyhow!("insert rune entry failed"));
        }

        Ok(())
    }
}
