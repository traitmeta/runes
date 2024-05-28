use anyhow::Ok;
use bitcoin::{consensus::Decodable, Transaction};

use super::*;
use std::{fs::File, io::BufReader};

fn read_mempool_dat(filename: &str) -> Result<(), anyhow::Error> {
    let mut file = File::open(filename)?;
    let mut reader = BufReader::new(file);

    let _ = Transaction::consensus_decode_from_finite_reader(&mut reader);
    Ok(())
}
