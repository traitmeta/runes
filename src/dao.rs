use super::*;

mod runes_entry;

pub trait RuneEntryDao {
    fn load(&mut self, id: &RuneId) -> Result<RuneEntry>;
    fn store(&mut self, id: &RuneId, entry: &RuneEntry) -> Result<()>;
    fn update(&mut self, id: &RuneId, entry: &RuneEntry) -> Result<()>;
    fn delete(&mut self, id: &RuneId) -> Result<()>;
}
