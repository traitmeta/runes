use super::*;

impl RuneEventDao for RuneMysqlDao {
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

    fn store_events(conn: &mut MysqlConnection, entity: &Vec<RuneEventEntity>) -> Result<()> {
        let insert_rows = diesel::insert_into(RuneEventTable)
            .values(entity)
            .execute(conn)
            .unwrap();

        if insert_rows == 0 {
            return Err(anyhow!("store_events failed"));
        }

        Ok(())
    }

    fn delete(conn: &mut MysqlConnection, id: &Txid) -> Result<()> {
        use crate::schema::rune_event::tx_id;
        let effect_rows = diesel::delete(RuneEventTable.filter(tx_id.eq(id.to_string())))
            .execute(conn)
            .expect("Error deleting rune entry");

        if effect_rows == 0 {
            return Err(anyhow!("delete event failed"));
        }

        Ok(())
    }
}
