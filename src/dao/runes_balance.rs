use super::*;

impl RuneBlanaceDao for RuneMysqlDao {
    fn load_by_outpoint(
        conn: &mut MysqlConnection,
        outpoint: &OutPoint,
    ) -> Result<Vec<RuneBalanceEntity>> {
        use self::schema::rune_balance::out_point;
        let results = RuneBalanceTable
            .filter(out_point.eq(outpoint.to_string()))
            .select(RuneBalanceEntity::as_select())
            .load(conn);

        match results {
            Ok(events) => Ok(events),
            Err(e) => Err(e.into()),
        }
    }

    fn update_spend_out_point(conn: &mut MysqlConnection, outpoint: &OutPoint) -> Result {
        use self::schema::rune_balance::{out_point, spent};

        let effect_rows =
            diesel::update(RuneBalanceTable.filter(out_point.eq(outpoint.to_string())))
                .set(spent.eq(true))
                .execute(conn)
                .expect("Error update rune entry");

        if effect_rows == 0 {
            return Err(anyhow!("update_spend_out_point failed"));
        }

        Ok(())
    }

    fn store_balances(conn: &mut MysqlConnection, entry: &Vec<RuneBalanceEntity>) -> Result<()> {
        let insert_rows = diesel::insert_into(RuneBalanceTable)
            .values(entry)
            .execute(conn)
            .unwrap();

        if insert_rows == 0 {
            return Err(anyhow!("store_balances failed"));
        }

        Ok(())
    }
}
