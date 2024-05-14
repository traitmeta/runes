use bitcoin::{script::Instruction, sighash::EcdsaSighashType};
use log::info;

use super::*;

pub(super) fn parse_tx(tx: &Transaction) -> Result<()> {
    for input in tx.input.iter() {
        let sig_hash_type = match input.script_sig.instructions().next() {
            Some(push) => match push {
                Ok(Instruction::PushBytes(b)) => {
                    let sig_hash_type = b[b.len() - 1];
                    Some(sig_hash_type)
                }
                _ => None,
            },
            None => None,
        };

        let sig_type = match sig_hash_type {
            Some(t) => Some(EcdsaSighashType::from_consensus(t as u32)),
            None => match input.witness.tapscript() {
                Some(s) => match s.instructions().next() {
                    Some(Ok(Instruction::PushBytes(b))) => {
                        let sig_hash_type = b[b.len() - 1];
                        Some(EcdsaSighashType::from_consensus(sig_hash_type as u32))
                    }
                    _ => None,
                },
                None => None,
            },
        };

        match sig_type {
            Some(t) => {
                if t == EcdsaSighashType::Single || t == EcdsaSighashType::None {
                    info!("Found! txid: {}, input: {:?}", tx.txid(), input);
                } else {
                    info!("Not Found! txid: {}, type: {:?}", tx.txid(), t);
                }
            }
            None => (),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use bitcoin::{consensus::Decodable, hashes, Transaction};

    use super::parse_tx;

    #[test]
    fn test_tx_witness_type_all() {
        env_logger::init();
        let raw_tx = <Vec<u8> as hashes::hex::FromHex>::from_hex("01000000000101b0e2d7ce5d23477af9eeb9c5a8f4863f94562b285916f2ca2d39d207d6fc43600000000000ffffffff022aa90d00000000001600146414fb09febc915551e92d90eaec1181faa8b258ac82030000000000160014b34e3991855417654e0ad20ad012e06480a457bb02483045022100b7da811a3bd23fdeb3d432361e1af07f0aa938844ecc1a8596828e46bbe5ed6a02204d02bd97fd26042fa75ea1f68f46e2f35e1b75909dae5710b6f768745fd6dca80121038de7eec284e374c0b0b38ce3d3672ba93f66c06c0131574feb5206dff61585de00000000").unwrap();
        let tx: Transaction = Decodable::consensus_decode(&mut raw_tx.as_slice()).unwrap();
        match parse_tx(&tx) {
            Ok(_) => {}
            Err(e) => println!("{}", e),
        }
    }

    #[test]
    fn test_tx_script_sig_type_all() {
        env_logger::init();
        let raw_tx = <Vec<u8> as hashes::hex::FromHex>::from_hex("01000000011cfd1d332444f740bcfe6bc80de53da6c54a111941faedf56c25bb68d129160c000000006a47304402203ff7162d6635246dbf59b7fa9e72e3023e959a73b1fbc51edbaaa5a8dbc6d2f70220776e2fa5740df01cc0ac47bda713e87fc59044960122ba45abb11c949655c584012103bc3c9134f5a5e3f08287d175d7e43368f72cb93a2e6cbb801b5e90d1ed628e60ffffffff01fa3b1d00000000001976a91429ad791e5913f9c4965ce084849ad7c810b4a07a88ac00000000").unwrap();
        let tx: Transaction = Decodable::consensus_decode(&mut raw_tx.as_slice()).unwrap();
        match parse_tx(&tx) {
            Ok(_) => {}
            Err(e) => println!("{}", e),
        }
    }


    #[test]
    fn test_tx_script_sig_type_all_plus_anyonecanpay() {
        env_logger::init();
        let raw_tx = <Vec<u8> as hashes::hex::FromHex>::from_hex("020000000001054c4d24324f889f17510326f1802fd9b67be69bf3178a7f732a3eae81a4fd250d0500000000ffffffff4c4d24324f889f17510326f1802fd9b67be69bf3178a7f732a3eae81a4fd250d0600000000ffffffffcbc703d0dcb717554ed55b7d59c1546b3599dde90d07dbfb199148a9919513360000000000ffffffff46832914defffe12ee6bf56a762b7d509c90cd0c4a33d8d7b3a942f516e00c160400000000ffffffff4c4d24324f889f17510326f1802fd9b67be69bf3178a7f732a3eae81a4fd250d0000000000ffffffff075802000000000000160014f84418c3e889f09f3598e164b221c52fda6ba6572202000000000000225120cb03645b892a156641d1bc09ad63aeb33dd7ab070e1a70ab798bf7e5a52f14ee405dc6000000000017a9142d8c0937fe2314b2543196a40a5fa47592315c1987e8fd00000000000016001449a0f76f14c5bc5dfdf7fc1d5dba6a9306c69a12632d3c0300000000225120cb03645b892a156641d1bc09ad63aeb33dd7ab070e1a70ab798bf7e5a52f14ee2c01000000000000160014f84418c3e889f09f3598e164b221c52fda6ba6572c01000000000000160014f84418c3e889f09f3598e164b221c52fda6ba6570247304402205cae7ab389d0de0442d45039dfe3130de3c9464f553e053899b92ee1267c10ec0220410bf90c6aafb6beb017058f9ec09d1c9d24ea53420da77bd3efeb3327d624d201210268b7232856bab37f5d30bffb47aaea1f652bc77f4191a55f4b88a786dd3b766a02483045022100a3135f3ad238b0255ba481f9d298d865a1f62f26e027503133f6827487e7812602201c56d39b5565f33f03ce82d2502f88f71fb6c5f755d0d99141e462f498bcbcee01210268b7232856bab37f5d30bffb47aaea1f652bc77f4191a55f4b88a786dd3b766a01414efafde2431539ee38f4d7e71e695fc5cb13fa55c8260c1170525ca595452cab3c3d69d034827a1f832907e2f1d2e636b51838e3f6512d04438ea377287bdd388301402c5914c7e92f2a025e61293c502112a0f059be60008a63c9e51733acba8610a80021b2e7a6614e958aeb35dd147c40094033137dcb8f6df28dac323a07ce6a6a0247304402205b7f54392e9c91587b61b45830dd262161a8192b5fa6d1075c9bfc67fe43786002202cc8575d2ba60d4c84952bb03bacff6054100b313d517dee544de03da56c9e2d01210268b7232856bab37f5d30bffb47aaea1f652bc77f4191a55f4b88a786dd3b766a00000000").unwrap();
        let tx: Transaction = Decodable::consensus_decode(&mut raw_tx.as_slice()).unwrap();
        match parse_tx(&tx) {
            Ok(_) => {}
            Err(e) => println!("{}", e),
        }
    }
}
