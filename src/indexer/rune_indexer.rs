use diesel::MysqlConnection;

use self::{
    dao::{RuneBlanaceDao, RuneEntryDao, RuneEventDao, RuneMysqlDao},
    entry::RuneEntry,
    event::Event,
    into_usize::IntoUsize,
    model::{RuneBalanceEntity, RuneEventEntity},
};

use super::*;

pub struct RuneIndexer<'client, 'conn> {
    pub block_time: u32,
    pub burned: HashMap<RuneId, Lot>,
    pub client: &'client Client,
    pub height: u32,
    pub minimum: Rune,
    pub runes: u64,
    pub conn: &'conn mut MysqlConnection,
}

impl<'client, 'conn> RuneIndexer<'client, 'conn> {
    pub fn parse_tx(&mut self, tx_index: u32, tx: &Transaction, txid: Txid) -> Result<()> {
        let artifact = Runestone::decipher(tx);
        let mut unallocated = self.unallocated(tx)?;
        let mut allocated: Vec<HashMap<RuneId, Lot>> = vec![HashMap::new(); tx.output.len()];
        let mut outpoint_to_balances: HashMap<OutPoint, Vec<(RuneId, Lot)>> = HashMap::new();
        let mut created_rune_entry: Option<(Txid, Artifact, RuneId, Rune)> = None;
        let mut runes_mints: Option<(RuneId, Lot)> = None;
        let mut events: Vec<Event> = Vec::new();
        if let Some(art) = &artifact {
            if let Some(id) = art.mint() {
                if let Some(amount) = self.mint(id, &mut runes_mints)? {
                    *unallocated.entry(id).or_default() += amount;
                    events.push(Event::RuneMinted {
                        amount: amount.n(),
                        block_height: self.height,
                        txid,
                        rune_id: id,
                    })
                }
            }

            let etched = self.etched(tx_index, tx, art)?;

            if let Artifact::Runestone(runestone) = art {
                if let Some((id, ..)) = etched {
                    *unallocated.entry(id).or_default() +=
                        runestone.etching.unwrap().premine.unwrap_or_default();

                    // Etch event
                    events.push(Event::RuneEtched {
                        block_height: self.height,
                        txid,
                        rune_id: id,
                    })
                }

                for Edict { id, amount, output } in runestone.edicts.iter().copied() {
                    let amount = Lot(amount);

                    // edicts with output values greater than the number of outputs
                    // should never be produced by the edict parser
                    let output = usize::try_from(output).unwrap();
                    assert!(output <= tx.output.len());

                    let id = if id == RuneId::default() {
                        let Some((id, ..)) = etched else {
                            continue;
                        };

                        id
                    } else {
                        id
                    };

                    let Some(balance) = unallocated.get_mut(&id) else {
                        continue;
                    };

                    let mut allocate = |balance: &mut Lot, amount: Lot, output: usize| {
                        if amount > 0 {
                            *balance -= amount;
                            *allocated[output].entry(id).or_default() += amount;
                        }
                    };

                    if output == tx.output.len() {
                        // find non-OP_RETURN outputs
                        let destinations = tx
                            .output
                            .iter()
                            .enumerate()
                            .filter_map(|(output, tx_out)| {
                                (!tx_out.script_pubkey.is_op_return()).then_some(output)
                            })
                            .collect::<Vec<usize>>();

                        if !destinations.is_empty() {
                            if amount == 0 {
                                // if amount is zero, divide balance between eligible outputs
                                let amount = *balance / destinations.len() as u128;
                                let remainder =
                                    usize::try_from(*balance % destinations.len() as u128).unwrap();

                                for (i, output) in destinations.iter().enumerate() {
                                    allocate(
                                        balance,
                                        if i < remainder { amount + 1 } else { amount },
                                        *output,
                                    );
                                }
                            } else {
                                // if amount is non-zero, distribute amount to eligible outputs
                                for output in destinations {
                                    allocate(balance, amount.min(*balance), output);
                                }
                            }
                        }
                    } else {
                        // Get the allocatable amount
                        let amount = if amount == 0 {
                            *balance
                        } else {
                            amount.min(*balance)
                        };

                        allocate(balance, amount, output);
                    }
                }
            }

            if let Some((id, rune)) = etched {
                created_rune_entry = Some((txid, art.clone(), id, rune));
            }
        }

        let mut burned: HashMap<RuneId, Lot> = HashMap::new();
        if let Some(Artifact::Cenotaph(_)) = &artifact {
            for (id, balance) in unallocated {
                *burned.entry(id).or_default() += balance;
            }
        } else {
            let pointer = &artifact
                .clone()
                .map(|art| match art {
                    Artifact::Runestone(runestone) => runestone.pointer,
                    Artifact::Cenotaph(_) => unreachable!(),
                })
                .unwrap_or_default();

            // assign all un-allocated runes to the default output, or the first non
            // OP_RETURN output if there is no default
            if let Some(vout) = pointer
                .map(|pointer| pointer.into_usize())
                .inspect(|&pointer| assert!(pointer < allocated.len()))
                .or_else(|| {
                    tx.output
                        .iter()
                        .enumerate()
                        .find(|(_vout, tx_out)| !tx_out.script_pubkey.is_op_return())
                        .map(|(vout, _tx_out)| vout)
                })
            {
                for (id, balance) in unallocated {
                    if balance > 0 {
                        *allocated[vout].entry(id).or_default() += balance;
                    }
                }
            } else {
                for (id, balance) in unallocated {
                    if balance > 0 {
                        *burned.entry(id).or_default() += balance;
                    }
                }
            }
        }

        // update outpoint balances
        for (vout, balances) in allocated.into_iter().enumerate() {
            if balances.is_empty() {
                continue;
            }

            let outpoint = OutPoint {
                txid,
                vout: vout.try_into().unwrap(),
            };

            // increment burned balances
            if tx.output[vout].script_pubkey.is_op_return() {
                for (id, balance) in &balances {
                    *burned.entry(*id).or_default() += *balance;
                }
                continue;
            }

            let mut balances = balances.into_iter().collect::<Vec<(RuneId, Lot)>>();

            // Sort balances by id so tests can assert balances in a fixed order
            balances.sort();

            for (id, balance) in balances {
                outpoint_to_balances
                    .entry(outpoint)
                    .and_modify(|balances| balances.push((id, balance)))
                    .or_insert(vec![(id, balance)]);

                // transfer event
                events.push(Event::RuneTransferred {
                    outpoint,
                    block_height: self.height,
                    txid,
                    rune_id: id,
                    amount: balance.0,
                })
            }
        }

        // increment entries with burned runes
        for (id, amount) in burned.iter() {
            // burn event
            events.push(Event::RuneBurned {
                block_height: self.height,
                txid,
                rune_id: *id,
                amount: amount.n(),
            })
        }

        let mut rune_entity: Option<(RuneId, RuneEntry)> = None;
        if let Some((txid, art, rune_id, rune)) = created_rune_entry {
            rune_entity = Some((rune_id, self.create_rune_entry(txid, art, rune_id, rune)?));
        };

        // TODO remove all this function have return's db opr
        let burned = self.find_all_burned_rune(&burned)?;
        // self.update(&burned, runes_mints)?;
        let event_entities = self.create_rune_event(events, tx, &artifact)?;
        let balance_entities = self.create_rune_balance(outpoint_to_balances, tx)?;
        self.store_all_at_once(
            rune_entity,
            event_entities,
            balance_entities,
            burned,
            runes_mints,
        );

        Ok(())
    }

    fn mint(&mut self, id: RuneId, mints: &mut Option<(RuneId, Lot)>) -> Result<Option<Lot>> {
        let mut rune_entry = match RuneMysqlDao::load_rune_entry(&mut self.conn, &id) {
            Ok(entry) => entry,
            Err(_) => return Ok(None),
        };

        let Ok(amount) = rune_entry.mintable(self.height.into()) else {
            return Ok(None);
        };

        rune_entry.mints += 1;

        *mints = Some((id, lot::Lot(rune_entry.mints)));
        // RuneMysqlDao::update_rune_mints(&mut self.conn, &id, rune_entry.mints)?;

        Ok(Some(Lot(amount)))
    }

    fn unallocated(&mut self, tx: &Transaction) -> Result<HashMap<RuneId, Lot>> {
        // map of rune ID to un-allocated balance of that rune
        let mut unallocated: HashMap<RuneId, Lot> = HashMap::new();

        // increment unallocated runes with the runes in tx inputs
        for input in &tx.input {
            match RuneMysqlDao::load_by_outpoint(&mut self.conn, &input.previous_output) {
                Ok(entry) => {
                    for event in entry.iter() {
                        let rune_id = RuneId::from_str(event.rune_id.as_str()).unwrap();
                        let a = BigDecimal::to_u128(&event.amount).unwrap();
                        *unallocated.entry(rune_id).or_default() += a;
                    }
                    if !entry.is_empty() {
                        RuneMysqlDao::update_spend_out_point(
                            &mut self.conn,
                            &input.previous_output,
                        )?;
                    }
                }
                Err(_) => {}
            }
        }

        Ok(unallocated)
    }

    // TODO use for calc_unallocated
    fn fast_unallocated(
        &mut self,
        tx: &Transaction,
    ) -> Result<(HashMap<RuneId, Lot>, HashMap<String, bool>)> {
        // map of rune ID to un-allocated balance of that rune
        let mut unallocated: HashMap<RuneId, Lot> = HashMap::new();
        let mut spent_outpoints = HashMap::new();
        let mut outpoints = Vec::new();
        for input in &tx.input {
            outpoints.push(input.previous_output.to_string());
        }

        match RuneMysqlDao::load_by_outpoints(&mut self.conn, outpoints) {
            Ok(entry) => {
                // increment unallocated runes with the runes in tx inputs
                for event in entry.iter() {
                    let rune_id = RuneId::from_str(event.rune_id.as_str()).unwrap();
                    let a = BigDecimal::to_u128(&event.amount).unwrap();
                    *unallocated.entry(rune_id).or_default() += a;
                    spent_outpoints.insert(event.out_point.clone(), true);
                }
            }
            Err(_) => {}
        }

        Ok((unallocated, spent_outpoints))
    }

    // TODO is not implemented
    pub fn write_all_todb_once(&mut self, spent_outpoints: HashMap<String, bool>) -> Result {
        let mut outpoints = Vec::new();
        for (k, v) in spent_outpoints.iter() {
            if *v {
                outpoints.push(k.clone());
            }
        }

        RuneMysqlDao::updates_spend_out_point(&mut self.conn, outpoints)
    }

    pub fn update(
        &mut self,
        burned: &HashMap<RuneId, Lot>,
        mints: Option<(RuneId, Lot)>,
    ) -> Result {
        for (rune_id, burn) in burned {
            let rune_entry = match RuneMysqlDao::load_rune_entry(&mut self.conn, &rune_id) {
                Ok(entry) => entry,
                Err(e) => return Err(e),
            };

            let bruned_value = rune_entry.burned.checked_add(burn.n()).unwrap();

            RuneMysqlDao::update_rune_burned(&mut self.conn, &rune_id, bruned_value)?;
        }

        if let Some((rune_id, mint)) = mints {
            RuneMysqlDao::update_rune_mints(&mut self.conn, &rune_id, mint.n())?;
        }

        Ok(())
    }

    fn etched(
        &mut self,
        tx_index: u32,
        tx: &Transaction,
        artifact: &Artifact,
    ) -> Result<Option<(RuneId, Rune)>> {
        let rune = match artifact {
            Artifact::Runestone(runestone) => match runestone.etching {
                Some(etching) => etching.rune,
                None => return Ok(None),
            },
            Artifact::Cenotaph(cenotaph) => match cenotaph.etching {
                Some(rune) => Some(rune),
                None => return Ok(None),
            },
        };

        let rune = if let Some(rune) = rune {
            let entry = match RuneMysqlDao::load_entry_by_rune(&mut self.conn, &rune) {
                Ok(entry) => Some(entry),
                Err(_) => None,
            };

            if rune < self.minimum
                || rune.is_reserved()
                || entry.is_some()
                || !self.tx_commits_to_rune(tx, rune)?
            {
                return Ok(None);
            }
            rune
        } else {
            Rune::reserved(self.height.into(), tx_index)
        };

        Ok(Some((
            RuneId {
                block: self.height.into(),
                tx: tx_index,
            },
            rune,
        )))
    }

    fn tx_commits_to_rune(&self, tx: &Transaction, rune: Rune) -> Result<bool> {
        let commitment = rune.commitment();

        for input in &tx.input {
            // extracting a tapscript does not indicate that the input being spent
            // was actually a taproot output. this is checked below, when we load the
            // output's entry from the database
            let Some(tapscript) = input.witness.tapscript() else {
                continue;
            };

            for instruction in tapscript.instructions() {
                // ignore errors, since the extracted script may not be valid
                let Ok(instruction) = instruction else {
                    break;
                };

                let Some(pushbytes) = instruction.push_bytes() else {
                    continue;
                };

                if pushbytes.as_bytes() != commitment {
                    continue;
                }

                let Some(tx_info) = self
                    .client
                    .get_raw_transaction_info(&input.previous_output.txid, None)
                    .into_option()?
                else {
                    panic!(
                        "can't get input transaction: {}",
                        input.previous_output.txid
                    );
                };

                let taproot = tx_info.vout[input.previous_output.vout.into_usize()]
                    .script_pub_key
                    .script()?
                    .is_v1_p2tr();

                if !taproot {
                    continue;
                }

                let commit_tx_height = self
                    .client
                    .get_block_header_info(&tx_info.blockhash.unwrap())
                    .into_option()?
                    .unwrap()
                    .height;

                let confirmations = self
                    .height
                    .checked_sub(commit_tx_height.try_into().unwrap())
                    .unwrap()
                    + 1;

                if confirmations >= Runestone::COMMIT_CONFIRMATIONS.into() {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    fn create_rune_entry(
        &mut self,
        txid: Txid,
        artifact: Artifact,
        id: RuneId,
        rune: Rune,
    ) -> Result<RuneEntry> {
        // self.rune_to_id.insert(rune.store(), id.store())?;
        // self.transaction_id_to_rune
        //     .insert(&txid.store(), rune.store())?;

        let number = self.runes;
        self.runes += 1;

        let entry = match artifact {
            Artifact::Cenotaph(_) => RuneEntry {
                block: id.block,
                burned: 0,
                divisibility: 0,
                etching: txid,
                terms: None,
                mints: 0,
                number,
                premine: 0,
                spaced_rune: SpacedRune { rune, spacers: 0 },
                symbol: None,
                timestamp: self.block_time.into(),
                turbo: false,
            },
            Artifact::Runestone(Runestone { etching, .. }) => {
                let Etching {
                    divisibility,
                    terms,
                    premine,
                    spacers,
                    symbol,
                    turbo,
                    ..
                } = etching.unwrap();

                RuneEntry {
                    block: id.block,
                    burned: 0,
                    divisibility: divisibility.unwrap_or_default(),
                    etching: txid,
                    terms,
                    mints: 0,
                    number,
                    premine: premine.unwrap_or_default(),
                    spaced_rune: SpacedRune {
                        rune,
                        spacers: spacers.unwrap_or_default(),
                    },
                    symbol,
                    timestamp: self.block_time.into(),
                    turbo,
                }
            }
        };

        // RuneMysqlDao::store_rune_entry(&mut self.conn, &id, &entry)?;

        Ok(entry)
    }

    fn create_rune_event(
        &mut self,
        events: Vec<Event>,
        tx: &Transaction,
        art: &Option<Artifact>,
    ) -> Result<Vec<RuneEventEntity>> {
        let mut entities: Vec<RuneEventEntity> = Vec::new();
        let art = art
            .as_ref()
            .map_or("".to_string(), |art| serde_json::to_string(art).unwrap());
        for event in events.iter() {
            match event {
                Event::RuneBurned {
                    amount,
                    block_height,
                    rune_id,
                    txid,
                } => {
                    let entity = RuneEventEntity {
                        id: 0,
                        block: *block_height as u64,
                        event_type: 4,
                        tx_id: txid.to_string(),
                        rune_id: rune_id.to_string(),
                        amount: BigDecimal::from_u128(*amount),
                        address: "".to_string(),
                        pk_script_hex: "".to_string(),
                        vout: 0,
                        rune_stone: art.clone(),
                        timestamp: self.block_time as u64,
                    };
                    entities.push(entity);
                }
                Event::RuneEtched {
                    block_height,
                    rune_id,
                    txid,
                } => {
                    let entity = RuneEventEntity {
                        id: 0,
                        block: *block_height as u64,
                        event_type: 1,
                        tx_id: txid.to_string(),
                        rune_id: rune_id.to_string(),
                        amount: None,
                        address: "".to_string(),
                        pk_script_hex: "".to_string(),
                        vout: 0,
                        rune_stone: art.clone(),
                        timestamp: self.block_time as u64,
                    };
                    entities.push(entity);
                }
                Event::RuneMinted {
                    amount,
                    block_height,
                    rune_id,
                    txid,
                } => {
                    let entity = RuneEventEntity {
                        id: 0,
                        block: *block_height as u64,
                        event_type: 2,
                        tx_id: txid.to_string(),
                        rune_id: rune_id.to_string(),
                        amount: BigDecimal::from_u128(*amount),
                        address: "".to_string(),
                        pk_script_hex: "".to_string(),
                        vout: 0,
                        rune_stone: art.clone(),
                        timestamp: self.block_time as u64,
                    };
                    entities.push(entity);
                }
                Event::RuneTransferred {
                    amount,
                    block_height,
                    outpoint,
                    rune_id,
                    txid,
                } => {
                    let addr = Address::from_script(
                        tx.output[outpoint.vout as usize].script_pubkey.as_script(),
                        Network::Bitcoin,
                    )
                    .unwrap();
                    let entity = RuneEventEntity {
                        id: 0,
                        block: *block_height as u64,
                        event_type: 3,
                        tx_id: txid.to_string(),
                        rune_id: rune_id.to_string(),
                        amount: BigDecimal::from_u128(*amount),
                        address: addr.to_string(),
                        pk_script_hex: tx.output[outpoint.vout as usize]
                            .script_pubkey
                            .to_hex_string(),
                        vout: outpoint.vout,
                        rune_stone: art.clone(),
                        timestamp: self.block_time as u64,
                    };
                    entities.push(entity);
                }
            }
        }

        // if entities.is_empty() {
        //     return Ok(entities);
        // }

        // RuneMysqlDao::store_events(&mut self.conn, &entities)?;
        Ok(entities)
    }

    fn create_rune_balance(
        &mut self,
        balances: HashMap<OutPoint, Vec<(RuneId, Lot)>>,
        tx: &Transaction,
    ) -> Result<Vec<RuneBalanceEntity>> {
        let mut entities: Vec<RuneBalanceEntity> = Vec::new();
        for (key, val) in balances.iter() {
            for (rune_id, lot) in val.iter() {
                let addr = Address::from_script(
                    tx.output[key.vout as usize].script_pubkey.as_script(),
                    Network::Bitcoin,
                )
                .unwrap();
                let entity = RuneBalanceEntity {
                    id: 0,
                    block: self.height as u64,
                    rune_id: rune_id.to_string(),
                    amount: BigDecimal::from_u128(lot.n()).unwrap(),
                    address: addr.to_string(),
                    pk_script_hex: tx.output[key.vout as usize].script_pubkey.to_hex_string(),
                    out_point: key.to_string(),
                    spent: false,
                };
                entities.push(entity);
            }
        }

        // if entities.is_empty() {
        //     return Ok(entities);
        // }

        // RuneMysqlDao::store_balances(&mut self.conn, &entities)?;
        Ok(entities)
    }

    // TODO use db transaction
    fn store_all_at_once(
        &mut self,
        rune_entity: Option<(RuneId, RuneEntry)>,
        event_entities: Vec<RuneEventEntity>,
        balance_entities: Vec<RuneBalanceEntity>,
        burned: Vec<RuneEntryEntity>,
        mints: Option<(RuneId, Lot)>,
    ) -> Result {
        match rune_entity {
            Some((id, entity)) => RuneMysqlDao::store_rune_entry(&mut self.conn, &id, &entity)?,
            None => {}
        };

        RuneMysqlDao::store_events(&mut self.conn, &event_entities)?;
        RuneMysqlDao::store_balances(&mut self.conn, &balance_entities)?;

        for burn in burned.iter() {
            let rune_id = RuneId::from_str(burn.rune_id.as_str()).unwrap();
            RuneMysqlDao::update_rune_burned(
                &mut self.conn,
                &rune_id,
                burn.burned.to_u128().unwrap(),
            )?;
        }

        if let Some((rune_id, mint)) = mints {
            RuneMysqlDao::update_rune_mints(&mut self.conn, &rune_id, mint.n())?;
        }

        Ok(())
    }

    pub fn find_all_burned_rune(
        &mut self,
        burned: &HashMap<RuneId, Lot>,
    ) -> Result<Vec<RuneEntryEntity>> {
        let mut rune_ids = Vec::new();
        let mut rune_id_val_map: HashMap<String, Lot> = HashMap::new();
        for (rune_id, burn) in burned.iter() {
            rune_ids.push(rune_id.to_string());
            rune_id_val_map.insert(rune_id.to_string(), *burn);
        }

        let mut runes_entry = match RuneMysqlDao::gets_rune_entry(&mut self.conn, rune_ids) {
            Ok(entry) => entry,
            Err(e) => return Err(e),
        };

        for rune in runes_entry.iter_mut() {
            let burn = rune_id_val_map.get(&rune.rune_id).unwrap();
            let burned = rune.burned.to_u128().unwrap();
            let bruned_value = burned.checked_add(burn.n()).unwrap();
            rune.burned = BigDecimal::from_u128(bruned_value).unwrap();
        }

        Ok(runes_entry)
    }
}
