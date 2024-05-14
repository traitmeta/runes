use super::*;

#[derive(Debug, PartialEq)]
pub enum MintError {
    Cap(u128),
    End(u64),
    Start(u64),
    Unmintable,
}

impl Display for MintError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            MintError::Cap(cap) => write!(f, "limited to {cap} mints"),
            MintError::End(end) => write!(f, "mint ended on block {end}"),
            MintError::Start(start) => write!(f, "mint starts on block {start}"),
            MintError::Unmintable => write!(f, "not mintable"),
        }
    }
}

pub(crate) fn encode_rune_balance(id: RuneId, balance: u128, buffer: &mut Vec<u8>) {
    varint::encode_to_vec(id.block.into(), buffer);
    varint::encode_to_vec(id.tx.into(), buffer);
    varint::encode_to_vec(balance, buffer);
}
