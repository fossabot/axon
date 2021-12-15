use std::{fmt, str::FromStr};

pub use ethereum_types::{
    Bloom, Public, Secret, Signature, H128, H160, H256, H512, H520, H64, U128, U256, U512,
};
use hasher::{Hasher as KeccakHasher, HasherKeccak};
use ophelia::{PublicKey, UncompressedPublicKey};
use ophelia_secp256k1::Secp256k1PublicKey;
use overlord::DurationConfig;
use serde::{de, Deserialize, Serialize};

use crate::types::{BlockNumber, Bytes, TypesError};
use crate::{ProtocolError, ProtocolResult};

lazy_static::lazy_static! {
    static ref HASHER_INST: HasherKeccak = HasherKeccak::new();
}

pub type Hash = H256;
pub type MerkleRoot = Hash;

const ADDRESS_LEN: usize = 20;

pub const NIL_DATA: H256 = H256([
    0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03, 0xc0,
    0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85, 0xa4, 0x70,
]);

pub const RLP_NULL: H256 = H256([
    0x56, 0xe8, 0x1f, 0x17, 0x1b, 0xcc, 0x55, 0xa6, 0xff, 0x83, 0x45, 0xe6, 0x92, 0xc0, 0xf8, 0x6e,
    0x5b, 0x48, 0xe0, 0x1b, 0x99, 0x6c, 0xad, 0xc0, 0x01, 0x62, 0x2f, 0xb5, 0xe3, 0x63, 0xb4, 0x21,
]);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DBBytes(pub Bytes);

impl AsRef<[u8]> for DBBytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

pub struct Hasher;

impl Hasher {
    pub fn digest<B: AsRef<[u8]>>(bytes: B) -> Hash {
        let hash = HASHER_INST.digest(bytes.as_ref());
        let mut ret = Hash::default();
        ret.0.copy_from_slice(&hash[0..32]);
        ret
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hex(String);

impl Hex {
    pub fn from_string(s: String) -> ProtocolResult<Self> {
        if (!s.starts_with("0x") && !s.starts_with("0X")) || s.len() < 3 {
            return Err(TypesError::HexPrefix.into());
        }

        hex::decode(&s[2..]).map_err(|error| TypesError::FromHex { error })?;
        Ok(Hex(s))
    }

    pub fn as_string(&self) -> String {
        self.0.to_owned()
    }

    pub fn as_string_trim0x(&self) -> String {
        (&self.0[2..]).to_owned()
    }

    pub fn decode(&self) -> Bytes {
        Bytes::from(hex::decode(&self.0[2..]).expect("impossible, already checked in from_string"))
    }
}

impl Default for Hex {
    fn default() -> Self {
        Hex::from_string("0x1".to_owned()).expect("Hex must start with 0x")
    }
}

impl Serialize for Hex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

struct HexVisitor;

impl<'de> de::Visitor<'de> for HexVisitor {
    type Value = Hex;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Expect a hex string")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Hex::from_string(v).map_err(|e| de::Error::custom(e.to_string()))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Hex::from_string(v.to_owned()).map_err(|e| de::Error::custom(e.to_string()))
    }
}

impl<'de> Deserialize<'de> for Hex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_string(HexVisitor)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Address(pub H160);

impl Default for Address {
    fn default() -> Self {
        Address::from_hex("0x0000000000000000000000000000000000000000")
            .expect("Address must consist of 20 bytes")
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_bytes(self.0.as_bytes())
    }
}

// struct AddressVisitor;

// impl<'de> de::Visitor<'de> for AddressVisitor {
//     type Value = Address;

//     fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//         formatter.write_str("Expect a bech32 string")
//     }

//     fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
//     where
//         E: de::Error,
//     {
//         Address::from_str(&v).map_err(|e| de::Error::custom(e.to_string()))
//     }

//     fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
//     where
//         E: de::Error,
//     {
//         Address::from_str(&v).map_err(|e| de::Error::custom(e.to_string()))
//     }
// }

// impl<'de> Deserialize<'de> for Address {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: de::Deserializer<'de>,
//     {
//         deserializer.deserialize_string(AddressVisitor)
//     }
// }

impl Address {
    pub fn from_pubkey_bytes<B: AsRef<[u8]>>(bytes: B) -> ProtocolResult<Self> {
        let compressed_pubkey_len = <Secp256k1PublicKey as PublicKey>::LENGTH;
        let uncompressed_pubkey_len = <Secp256k1PublicKey as UncompressedPublicKey>::LENGTH;

        let slice = bytes.as_ref();
        if slice.len() != compressed_pubkey_len && slice.len() != uncompressed_pubkey_len {
            return Err(TypesError::InvalidPublicKey.into());
        }

        // Drop first byte
        let hash = {
            if slice.len() == compressed_pubkey_len {
                let pubkey = Secp256k1PublicKey::try_from(slice)
                    .map_err(|_| TypesError::InvalidPublicKey)?;
                Hasher::digest(&(pubkey.to_uncompressed_bytes())[1..])
            } else {
                Hasher::digest(&slice[1..])
            }
        };

        Ok(Self::from_hash(hash))
    }

    pub fn from_hash(hash: Hash) -> Self {
        Self(H160::from_slice(&hash.as_bytes()[12..]))
    }

    pub fn from_bytes(bytes: Bytes) -> ProtocolResult<Self> {
        ensure_len(bytes.len(), ADDRESS_LEN)?;
        Ok(Self(H160::from_slice(&bytes.as_ref()[0..20])))
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn from_hex(s: &str) -> ProtocolResult<Self> {
        let s = clean_0x(s)?;
        let bytes = hex::decode(s).map_err(TypesError::from)?;

        let bytes = Bytes::from(bytes);
        Self::from_bytes(bytes)
    }

    pub fn eip55(&self) -> String {
        self.to_string()
    }
}

impl FromStr for Address {
    type Err = ProtocolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if checksum(s) != s {
            return Err(TypesError::InvalidCheckSum.into());
        }

        Address::from_hex(&s.to_lowercase())
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let eip55 = checksum(&hex::encode(&self.0));
        eip55.fmt(f)?;
        Ok(())
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let eip55 = checksum(&hex::encode(&self.0));
        eip55.fmt(f)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, Copy, PartialEq, Eq)]
pub struct MetadataVersion {
    pub start: BlockNumber,
    pub end:   BlockNumber,
}

impl MetadataVersion {
    pub fn new(start: BlockNumber, end: BlockNumber) -> Self {
        MetadataVersion { start, end }
    }

    pub fn contains(&self, number: BlockNumber) -> bool {
        self.start <= number && number < self.end
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub chain_id:        U256,
    pub version:         MetadataVersion,
    pub common_ref:      Hex,
    pub timeout_gap:     u64,
    pub gas_limit:       u64,
    pub gas_price:       u64,
    pub interval:        u64,
    pub verifier_list:   Vec<ValidatorExtend>,
    pub propose_ratio:   u64,
    pub prevote_ratio:   u64,
    pub precommit_ratio: u64,
    pub brake_ratio:     u64,
    pub tx_num_limit:    u64,
    pub max_tx_size:     u64,
}

impl From<Metadata> for DurationConfig {
    fn from(m: Metadata) -> Self {
        DurationConfig {
            propose_ratio:   m.propose_ratio,
            prevote_ratio:   m.prevote_ratio,
            precommit_ratio: m.precommit_ratio,
            brake_ratio:     m.brake_ratio,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Validator {
    pub pub_key:        Bytes,
    pub propose_weight: u32,
    pub vote_weight:    u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct ValidatorExtend {
    pub bls_pub_key:    Hex,
    pub pub_key:        Hex,
    pub address:        H160,
    pub propose_weight: u32,
    pub vote_weight:    u32,
}

impl From<ValidatorExtend> for Validator {
    fn from(ve: ValidatorExtend) -> Self {
        Validator {
            pub_key:        ve.pub_key.decode(),
            propose_weight: ve.propose_weight,
            vote_weight:    ve.vote_weight,
        }
    }
}

impl std::fmt::Debug for ValidatorExtend {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let bls_pub_key = self.bls_pub_key.as_string_trim0x();
        let pk = if bls_pub_key.len() > 8 {
            unsafe { bls_pub_key.get_unchecked(0..8) }
        } else {
            bls_pub_key.as_str()
        };

        write!(
            f,
            "bls public key {:?}, public key {:?}, address {:?} propose weight {}, vote weight {}",
            pk, self.pub_key, self.address, self.propose_weight, self.vote_weight
        )
    }
}

fn ensure_len(real: usize, expect: usize) -> ProtocolResult<()> {
    if real != expect {
        Err(TypesError::LengthMismatch { expect, real }.into())
    } else {
        Ok(())
    }
}

fn clean_0x(s: &str) -> ProtocolResult<&str> {
    if s.starts_with("0x") || s.starts_with("0X") {
        Ok(&s[2..])
    } else {
        Err(TypesError::HexPrefix.into())
    }
}

pub fn checksum(address: &str) -> String {
    let address = address.trim_start_matches("0x").to_lowercase();
    let address_hash = hex::encode(Hasher::digest(address.as_bytes()));

    address
        .char_indices()
        .fold(String::from("0x"), |mut acc, (index, address_char)| {
            // this cannot fail since it's Keccak256 hashed
            let n = u16::from_str_radix(&address_hash[index..index + 1], 16).unwrap();

            if n > 7 {
                // make char uppercase if ith character is 9..f
                acc.push_str(&address_char.to_uppercase().to_string())
            } else {
                // already lowercased
                acc.push(address_char)
            }

            acc
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eip55() {
        let addr = "0x35e70c3f5a794a77efc2ec5ba964bffcc7fd2c0a";
        let eip55 = Address::from_hex(addr).unwrap();
        assert_eq!(
            eip55.to_string(),
            "0x35E70C3F5A794A77Efc2Ec5bA964BFfcC7Fd2C0a"
        );
    }
}