use self::{
    dao::RuneEntryDao,
    entry::{
        Entry, InscriptionIdValue, OutPointValue, RuneEntry, RuneEntryValue, RuneIdValue, TxidValue,
    },
};

use super::*;

pub(super) struct RuneIndexer<'client, T: RuneEntryDao> {
    pub(super) block_time: u32,
    pub(super) burned: HashMap<RuneId, Lot>,
    pub(super) client: &'client Client,
    pub(super) height: u32,
    pub(super) minimum: Rune,
    pub(super) runes: u64,
    pub(super) dao: T,
}

pub struct HotRunes {
    times: u64,
    spaced_rune: SpacedRune,
    tx_ids: Vec<Txid>,
}

impl<'client, T> RuneIndexer<'client, T>
where
    T: RuneEntryDao,
{
    pub(super) fn index_runes(
        &mut self,
        tx_index: u32,
        tx: &Transaction,
        txid: Txid,
        mut runes_map: HashMap<RuneId, HotRunes>,
    ) -> Result<()> {
        let artifact = Runestone::decipher(tx);
        if let Some(artifact) = &artifact {
            if let Some(id) = artifact.mint() {
                let mut rune = runes_map.get_mut(&id).unwrap();
                rune.times += 1;
                rune.tx_ids.push(txid);
            }
        }

        Ok(())
    }

    fn mint(&mut self, id: RuneId) -> Result<Option<Lot>> {
        let mut rune_entry = match self.dao.load(&id) {
            Ok(entry) => entry,
            Err(_) => return Ok(None),
        };

        let Ok(amount) = rune_entry.mintable(self.height.into()) else {
            return Ok(None);
        };

        rune_entry.mints += 1;

        self.dao.update(&id, &rune_entry)?;

        Ok(Some(Lot(amount)))
    }
}
