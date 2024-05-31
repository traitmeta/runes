use {
    super::{fetcher::Fetcher, *},
    crate::dao::{RuneEntryDao, RuneMysqlDao},
    bitcoincore_rpc::Auth,
    diesel::MysqlConnection,
    futures::future::try_join_all,
    std::sync::mpsc,
    tokio::sync::mpsc::{error::TryRecvError, Receiver, Sender},
};

pub(crate) struct BlockData {
    pub(crate) header: Header,
    pub(crate) txdata: Vec<(Transaction, Txid)>,
}

impl From<Block> for BlockData {
    fn from(block: Block) -> Self {
        BlockData {
            header: block.header,
            txdata: block
                .txdata
                .into_iter()
                .map(|transaction| {
                    let txid = transaction.txid();
                    (transaction, txid)
                })
                .collect(),
        }
    }
}

pub(crate) struct Updater<'client> {
    pub(super) height: u32,
    pub(super) client: &'client Client,
    pub(super) conn: MysqlConnection,
}

impl<'index> Updater<'index> {
    pub(crate) fn update_index(
        &mut self,
        bitcoin_rpc_url: &str,
        user: &str,
        passwd: &str,
    ) -> Result {
        let rx = Self::fetch_blocks_from(bitcoin_rpc_url, user, passwd, self.height)?;

        let (mut outpoint_sender, mut value_receiver) = Self::spawn_fetcher(
            bitcoin_rpc_url,
            Auth::UserPass(user.to_string(), passwd.to_string()),
        )?;

        while let Ok(block) = rx.recv() {
            self.index_block(&mut outpoint_sender, &mut value_receiver, block)?;

            if SHUTTING_DOWN.load(atomic::Ordering::Relaxed) {
                break;
            }
        }

        Ok(())
    }

    fn fetch_blocks_from(
        bitcoin_rpc_url: &str,
        name: &str,
        passwd: &str,
        mut height: u32,
    ) -> Result<mpsc::Receiver<BlockData>> {
        let (tx, rx) = mpsc::sync_channel(32);

        let client = Client::new(
            bitcoin_rpc_url,
            Auth::UserPass(name.to_string(), passwd.to_string()),
        )
        .with_context(|| format!("failed to connect to Bitcoin Core RPC at `{bitcoin_rpc_url}`"))?;

        thread::spawn(move || loop {
            match Self::get_block_with_retries(&client, height) {
                Ok(Some(block)) => {
                    if let Err(err) = tx.send(block.into()) {
                        log::info!("Block receiver disconnected: {err}");
                        break;
                    }
                    height += 1;
                }
                Ok(None) => break,
                Err(err) => {
                    log::error!("failed to fetch block {height}: {err}");
                    break;
                }
            }
        });

        Ok(rx)
    }

    fn get_block_with_retries(client: &Client, height: u32) -> Result<Option<Block>> {
        let mut errors = 0;
        loop {
            match client
                .get_block_hash(height.into())
                .into_option()
                .and_then(|option| option.map(|hash| Ok(client.get_block(&hash)?)).transpose())
            {
                Err(err) => {
                    if cfg!(test) {
                        return Err(err);
                    }

                    errors += 1;
                    let seconds = 1 << errors;
                    log::warn!("failed to fetch block {height}, retrying in {seconds}s: {err}");

                    if seconds > 120 {
                        log::error!("would sleep for more than 120s, giving up");
                        return Err(err);
                    }

                    thread::sleep(Duration::from_secs(seconds));
                }
                Ok(result) => return Ok(result),
            }
        }
    }

    fn spawn_fetcher(
        bitcoin_rpc_url: &str,
        auth: Auth,
    ) -> Result<(Sender<OutPoint>, Receiver<u64>)> {
        let fetcher = Fetcher::new(bitcoin_rpc_url, auth)?;

        // Not sure if any block has more than 20k inputs, but none so far after first inscription block
        const CHANNEL_BUFFER_SIZE: usize = 20_000;
        let (outpoint_sender, mut outpoint_receiver) =
            tokio::sync::mpsc::channel::<OutPoint>(CHANNEL_BUFFER_SIZE);
        let (value_sender, value_receiver) = tokio::sync::mpsc::channel::<u64>(CHANNEL_BUFFER_SIZE);

        // Batch 2048 missing inputs at a time. Arbitrarily chosen for now, maybe higher or lower can be faster?
        // Did rudimentary benchmarks with 1024 and 4096 and time was roughly the same.
        const BATCH_SIZE: usize = 2048;
        // Default rpcworkqueue in bitcoind is 16, meaning more than 16 concurrent requests will be rejected.
        // Since we are already requesting blocks on a separate thread, and we don't want to break if anything
        // else runs a request, we keep this to 12.
        const PARALLEL_REQUESTS: usize = 12;

        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                loop {
                    let Some(outpoint) = outpoint_receiver.recv().await else {
                        log::debug!("Outpoint channel closed");
                        return;
                    };
                    // There's no try_iter on tokio::sync::mpsc::Receiver like std::sync::mpsc::Receiver.
                    // So we just loop until BATCH_SIZE doing try_recv until it returns None.
                    let mut outpoints = vec![outpoint];
                    for _ in 0..BATCH_SIZE - 1 {
                        let Ok(outpoint) = outpoint_receiver.try_recv() else {
                            break;
                        };
                        outpoints.push(outpoint);
                    }
                    // Break outpoints into chunks for parallel requests
                    let chunk_size = (outpoints.len() / PARALLEL_REQUESTS) + 1;
                    let mut futs = Vec::with_capacity(PARALLEL_REQUESTS);
                    for chunk in outpoints.chunks(chunk_size) {
                        let txids = chunk.iter().map(|outpoint| outpoint.txid).collect();
                        let fut = fetcher.get_transactions(txids);
                        futs.push(fut);
                    }
                    let txs = match try_join_all(futs).await {
                        Ok(txs) => txs,
                        Err(e) => {
                            log::error!("Couldn't receive txs {e}");
                            return;
                        }
                    };
                    // Send all tx output values back in order
                    for (i, tx) in txs.iter().flatten().enumerate() {
                        let Ok(_) = value_sender
                            .send(tx.output[usize::try_from(outpoints[i].vout).unwrap()].value)
                            .await
                        else {
                            log::error!("Value channel closed unexpectedly");
                            return;
                        };
                    }
                }
            })
        });

        Ok((outpoint_sender, value_receiver))
    }

    fn index_block(
        &mut self,
        outpoint_sender: &mut Sender<OutPoint>,
        value_receiver: &mut Receiver<u64>,
        block: BlockData,
    ) -> Result<()> {
        let start = Instant::now();
        log::info!(
            "Block {} at {} with {} transactions…",
            self.height,
            timestamp(block.header.time.into()),
            block.txdata.len()
        );

        // If value_receiver still has values something went wrong with the last block
        // Could be an assert, shouldn't recover from this and commit the last block
        let Err(TryRecvError::Empty) = value_receiver.try_recv() else {
            return Err(anyhow!("Previous block did not consume all input values"));
        };

        let start_heigth = Rune::first_rune_height(Network::Bitcoin);
        if self.height >= start_heigth {
            let gets_rune_number = RuneMysqlDao::gets_rune_number(&mut self.conn);
            let mut rune_updater = RuneIndexer {
                block_time: block.header.time,
                burned: HashMap::new(),
                client: &self.client,
                height: self.height,
                minimum: Rune::minimum_at_height(Network::Bitcoin, Height(self.height)),
                runes: gets_rune_number.map_or(0, |f| f + 1),
                conn: &mut self.conn,
            };

            for (i, (tx, txid)) in block.txdata.iter().enumerate() {
                rune_updater.parse_tx(u32::try_from(i).unwrap(), tx, *txid)?;
            }
        }

        self.height += 1;

        log::info!("index runes in {} ms", (Instant::now() - start).as_millis(),);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::dao::new_db_conn;
    use anyhow::Context;
    use bitcoincore_rpc::{Auth, Client};
    use dotenv::dotenv;
    use std::env;

    use super::Updater;

    #[test]
    fn test_runes() {
        env_logger::init();
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").unwrap();
        let bitcoin_url = env::var("BITCOIN_URL").unwrap();
        let bitcoin_user = env::var("BITCOIN_USER").unwrap();
        let bitcoin_passwd = env::var("BITCOIN_PASSWD").unwrap();
        let conn = new_db_conn(database_url.as_str());
        let client = Client::new(
            bitcoin_url.as_str(),
            Auth::UserPass(bitcoin_user, bitcoin_passwd),
        )
        .with_context(|| format!("failed to connect to Bitcoin Core RPC"))
        .unwrap();

        let mut updater = Updater {
            height: 840000,
            client: &client,
            conn: conn,
        };

        match updater.update_index("192.168.103.162:8332", "foo", "TQlDLNY6eJzZ5fYw") {
            Ok(_) => {}
            Err(e) => {
                println!("{}", e);
            }
        }
    }
}
