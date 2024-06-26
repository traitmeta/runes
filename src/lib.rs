#![allow(
    clippy::large_enum_variant,
    clippy::result_large_err,
    clippy::too_many_arguments,
    clippy::type_complexity
)]
#![deny(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use {
    anyhow::{anyhow, bail, ensure, Context, Error},
    bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive},
    bip39::Mnemonic,
    bitcoin::{
        address::{Address, NetworkUnchecked},
        block::Header,
        blockdata::{
            constants::{DIFFCHANGE_INTERVAL, MAX_SCRIPT_ELEMENT_SIZE, SUBSIDY_HALVING_INTERVAL},
            locktime::absolute::LockTime,
        },
        consensus::{self, Decodable, Encodable},
        hash_types::{BlockHash, TxMerkleNode},
        hashes::Hash,
        script, Amount, Block, Network, OutPoint, Script, ScriptBuf, Sequence, Transaction, TxIn,
        TxOut, Txid, Witness,
    },
    bitcoincore_rpc::{Client, RpcApi},
    chrono::{DateTime, TimeZone, Utc},
    ciborium::Value,
    clap::{ArgGroup, Parser},
    html_escaper::{Escape, Trusted},
    http::HeaderMap,
    lazy_static::lazy_static,
    ordinals::{
        varint, Artifact, Charm, Edict, Epoch, Etching, Height, Pile, Rarity, Rune, RuneId,
        Runestone, Sat, SatPoint, SpacedRune, Terms,
    },
    regex::Regex,
    reqwest::Url,
    serde::{Deserialize, Deserializer, Serialize},
    serde_with::{DeserializeFromStr, SerializeDisplay},
    std::{
        cmp::{self, Reverse},
        collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
        env,
        fmt::{self, Display, Formatter},
        fs,
        io::{self, Cursor, Read},
        mem,
        net::ToSocketAddrs,
        path::{Path, PathBuf},
        process::{self, Command, Stdio},
        str::FromStr,
        sync::{
            atomic::{self, AtomicBool},
            Arc, Mutex,
        },
        thread,
        time::{Duration, Instant, SystemTime},
    },
    sysinfo::System,
    tokio::{runtime::Runtime, task},
};

pub(crate) use self::{entry::RuneEntry, model::RuneEntryEntity};
pub use self::{
    indexer::{Lot, MintError, RuneIndexer},
    schema::etching as EtchingTable,
    schema::rune_balance::dsl::rune_balance as RuneBalanceTable,
    schema::rune_entry::dsl::rune_entry as RuneEntryTable,
    schema::rune_event::dsl::rune_event as RuneEventTable,
};
pub use ordinals::InscriptionId;

mod dao;
mod entry;
mod fetcher;
mod indexer;
mod model;
pub mod schema;
mod updater;
mod mempool;

type Result<T = (), E = Error> = std::result::Result<T, E>;

const TARGET_POSTAGE: Amount = Amount::from_sat(10_000);

static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);
static LISTENERS: Mutex<Vec<axum_server::Handle>> = Mutex::new(Vec::new());
static INDEXER: Mutex<Option<thread::JoinHandle<()>>> = Mutex::new(None);

pub fn timestamp(seconds: u64) -> DateTime<Utc> {
    Utc.timestamp_opt(seconds.try_into().unwrap_or(i64::MAX), 0)
        .unwrap()
}

fn target_as_block_hash(target: bitcoin::Target) -> BlockHash {
    BlockHash::from_raw_hash(Hash::from_byte_array(target.to_le_bytes()))
}

fn unbound_outpoint() -> OutPoint {
    OutPoint {
        txid: Hash::all_zeros(),
        vout: 0,
    }
}

fn uncheck(address: &Address) -> Address<NetworkUnchecked> {
    address.to_string().parse().unwrap()
}

fn default<T: Default>() -> T {
    Default::default()
}

fn gracefully_shutdown_indexer() {
    if let Some(indexer) = INDEXER.lock().unwrap().take() {
        // We explicitly set this to true to notify the thread to not take on new work
        SHUTTING_DOWN.store(true, atomic::Ordering::Relaxed);
        log::info!("Waiting for index thread to finish...");
        if indexer.join().is_err() {
            log::warn!("Index thread panicked; join failed");
        }
    }
}

pub(crate) trait BitcoinCoreRpcResultExt<T> {
    fn into_option(self) -> Result<Option<T>>;
}

impl<T> BitcoinCoreRpcResultExt<T> for Result<T, bitcoincore_rpc::Error> {
    fn into_option(self) -> Result<Option<T>> {
        match self {
            Ok(ok) => Ok(Some(ok)),
            Err(bitcoincore_rpc::Error::JsonRpc(bitcoincore_rpc::jsonrpc::error::Error::Rpc(
                bitcoincore_rpc::jsonrpc::error::RpcError { code: -8, .. },
            ))) => Ok(None),
            Err(bitcoincore_rpc::Error::JsonRpc(bitcoincore_rpc::jsonrpc::error::Error::Rpc(
                bitcoincore_rpc::jsonrpc::error::RpcError { message, .. },
            ))) if message.ends_with("not found") => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
