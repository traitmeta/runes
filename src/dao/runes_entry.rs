use super::*;
use bigdecimal::BigDecimal;
use bigdecimal::ToPrimitive;
use diesel::MysqlConnection;
use diesel::QueryDsl;
use futures::future::ok;

pub fn convert_rune_entry_to_model(rune_id: &RuneId, runes_entry: &RuneEntry) -> RuneEntryEntity {
    let mut entity = RuneEntryEntity {
        id: 0u64,
        block: runes_entry.block,
        burned: BigDecimal::from(runes_entry.burned),
        divisibility: runes_entry.divisibility,
        etching: runes_entry.etching.to_string(),
        mints: BigDecimal::from(runes_entry.mints),
        number: runes_entry.number,
        premine: BigDecimal::from(runes_entry.premine),
        spaced_rune: runes_entry.spaced_rune.to_string(),
        rune_id: rune_id.to_string(),
        rune: BigDecimal::from(runes_entry.spaced_rune.rune.0),
        timestamp: runes_entry.timestamp,
        symbol: runes_entry.symbol.unwrap_or('¤').to_string(),
        turbo: runes_entry.turbo,
        amount: None,
        cap: None,
        height_start: None,
        height_end: None,
        offset_start: None,
        offset_end: None,
    };

    if let Some(terms) = runes_entry.terms {
        entity.amount = terms.amount.map(|a| BigDecimal::from(a));
        entity.cap = terms.cap.map(|a| BigDecimal::from(a));
        entity.height_start = terms.height.0;
        entity.height_end = terms.height.1;
        entity.offset_start = terms.offset.0;
        entity.offset_end = terms.offset.1;
    }

    entity
}

pub fn convert_model_to_rune_entry(entity: &RuneEntryEntity) -> RuneEntry {
    let terms = Terms {
        amount: match &entity.amount {
            Some(a) => a.to_u128(),
            None => None,
        },
        cap: match &entity.cap {
            Some(a) => a.to_u128(),
            None => None,
        },
        height: (entity.height_start, entity.height_end),
        offset: (entity.offset_start, entity.offset_end),
    };

    let rune_entry = RuneEntry {
        block: entity.block as u64,
        burned: entity.burned.to_u128().unwrap(),
        divisibility: entity.divisibility,
        etching: Txid::from_str(entity.etching.as_str()).unwrap(),
        mints: entity.mints.to_u128().unwrap(),
        number: entity.number,
        premine: entity.premine.to_u128().unwrap(),
        spaced_rune: SpacedRune::from_str(entity.spaced_rune.as_str()).unwrap(),
        symbol: entity.symbol.chars().last(),
        terms: Some(terms),
        timestamp: entity.timestamp,
        turbo: entity.turbo,
    };

    rune_entry
}

impl RuneEntryDao for RuneMysqlDao {
    fn gets_rune_entry(
        conn: &mut MysqlConnection,
        ids: Vec<String>,
    ) -> Result<Vec<RuneEntryEntity>> {
        use self::schema::rune_entry::rune_id;

        let results = RuneEntryTable
            .filter(rune_id.eq_any(ids))
            .select(RuneEntryEntity::as_select())
            .load(conn);

        match results {
            Ok(entities) => Ok(entities),
            Err(e) => Err(e.into()),
        }
    }

    fn gets_rune_number(conn: &mut MysqlConnection) -> Option<u64> {
        use self::schema::rune_entry::number;

        let result = RuneEntryTable
            .order(number.desc())
            .select(number)
            .first::<u64>(conn);

        match result {
            Ok(entities) => Some(entities),
            Err(_) => None,
        }
    }

    fn load_rune_entry(conn: &mut MysqlConnection, id: &RuneId) -> Result<RuneEntry> {
        use self::schema::rune_entry::rune_id;

        let result = RuneEntryTable
            .filter(rune_id.eq(id.to_string()))
            .select(RuneEntryEntity::as_select())
            .first(conn);

        match result {
            Ok(entity) => {
                let rune_entry = convert_model_to_rune_entry(&entity);
                Ok(rune_entry)
            }
            Err(e) => Err(e.into()),
        }
    }

    fn load_entry_by_rune(conn: &mut MysqlConnection, _rune: &Rune) -> Result<RuneEntry> {
        use self::schema::rune_entry::rune;

        let result = RuneEntryTable
            .filter(rune.eq(BigDecimal::from_u128(_rune.n()).unwrap()))
            .select(RuneEntryEntity::as_select())
            .first(conn);

        match result {
            Ok(entity) => {
                let rune_entry = convert_model_to_rune_entry(&entity);
                Ok(rune_entry)
            }
            Err(e) => Err(e.into()),
        }
    }

    fn store_rune_entry(conn: &mut MysqlConnection, id: &RuneId, entry: &RuneEntry) -> Result<()> {
        let entity = convert_rune_entry_to_model(id, entry);
        let insert_rows = diesel::insert_into(RuneEntryTable)
            .values(&entity)
            .execute(conn)
            .unwrap();

        if insert_rows == 0 {
            return Err(anyhow!("insert rune entry failed"));
        }

        Ok(())
    }

    fn update_rune_mints(conn: &mut MysqlConnection, id: &RuneId, _mints: u128) -> Result<()> {
        use self::schema::rune_entry::{mints, rune_id};

        let effect_rows = diesel::update(RuneEntryTable.filter(rune_id.eq(id.to_string())))
            .set(mints.eq(BigDecimal::from(_mints)))
            .execute(conn)
            .expect("Error update rune entry");

        if effect_rows == 0 {
            return Err(anyhow!("insert rune entry failed"));
        }

        Ok(())
    }

    fn update_rune_burned(conn: &mut MysqlConnection, id: &RuneId, _burned: u128) -> Result<()> {
        use self::schema::rune_entry::{burned, rune_id};

        let effect_rows = diesel::update(RuneEntryTable.filter(rune_id.eq(id.to_string())))
            .set(burned.eq(BigDecimal::from(_burned)))
            .execute(conn)
            .expect("Error update rune entry");

        if effect_rows == 0 {
            return Err(anyhow!("insert rune entry failed"));
        }

        Ok(())
    }

    fn delete_rune_entry(conn: &mut MysqlConnection, id: &RuneId) -> Result<()> {
        use self::schema::rune_entry::rune_id;
        let effect_rows = diesel::delete(RuneEntryTable.filter(rune_id.eq(id.to_string())))
            .execute(conn)
            .expect("Error deleting rune entry");

        if effect_rows == 0 {
            return Err(anyhow!("insert rune entry failed"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        dao::{new_db_conn, runes_entry::RuneMysqlDao, RuneEntryDao},
        RuneEntry,
    };
    use ordinals::{inscription_id::txid, SpacedRune, Terms};
    use std::str::FromStr;

    #[test]
    fn load_not_found_should_not_err() {
        let mut conn = new_db_conn("mysql://meta:meta@localhost:3306/runes");
        assert!(RuneMysqlDao::load_rune_entry(
            &mut conn,
            &super::RuneId::from_str("123:1").unwrap()
        )
        .is_err());
    }

    #[test]
    fn load_found_should_be_ok() {
        let mut conn = new_db_conn("mysql://meta:meta@localhost:3306/runes");
        assert!(RuneMysqlDao::load_rune_entry(
            &mut conn,
            &super::RuneId::from_str("123:1").unwrap()
        )
        .is_err());
        let entry = RuneEntry {
            block: 123,
            burned: 0,
            divisibility: 0,
            etching: txid(1),
            mints: 0,
            number: 1,
            premine: 100,
            spaced_rune: SpacedRune::from_str("FUNCTION.TEST").unwrap(),
            symbol: Some('L'),
            terms: Some(Terms {
                amount: Some(100),
                cap: Some(100),
                height: (Some(100), None),
                offset: (None, None),
            }),
            timestamp: 0,
            turbo: false,
        };
        assert!(RuneMysqlDao::store_rune_entry(
            &mut conn,
            &super::RuneId::from_str("123:1").unwrap(),
            &entry
        )
        .is_ok());

        assert!(RuneMysqlDao::load_rune_entry(
            &mut conn,
            &super::RuneId::from_str("123:1").unwrap()
        )
        .is_ok());

        assert!(RuneMysqlDao::delete_rune_entry(
            &mut conn,
            &super::RuneId::from_str("123:1").unwrap()
        )
        .is_ok());
    }
}
