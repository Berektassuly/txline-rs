//! Proof decoding and Anchor-compatible DTO conversion helpers.

use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE, URL_SAFE_NO_PAD};
use serde::de::{Error as DeError, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Result, TxlineError};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash32([u8; 32]);

impl Hash32 {
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let bytes = bytes.as_ref();
        if bytes.len() != 32 {
            return Err(TxlineError::proof_decode(format!(
                "expected 32 bytes, received {}",
                bytes.len()
            )));
        }
        let mut out = [0u8; 32];
        out.copy_from_slice(bytes);
        Ok(Self(out))
    }

    pub fn decode(value: &str) -> Result<Self> {
        let value = value.trim();
        if value.is_empty() {
            return Err(TxlineError::proof_decode("hash string must not be empty"));
        }

        let hex_candidate = value.strip_prefix("0x").unwrap_or(value);
        if hex_candidate.len() == 64 && hex_candidate.chars().all(|c| c.is_ascii_hexdigit()) {
            return Self::decode_hex(hex_candidate);
        }

        STANDARD
            .decode(value)
            .or_else(|_| URL_SAFE.decode(value))
            .or_else(|_| URL_SAFE_NO_PAD.decode(value))
            .map_err(TxlineError::from)
            .and_then(Self::from_bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    fn decode_hex(value: &str) -> Result<Self> {
        let mut bytes = [0u8; 32];
        for (idx, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
            let hex = std::str::from_utf8(chunk)
                .map_err(|err| TxlineError::proof_decode(err.to_string()))?;
            bytes[idx] = u8::from_str_radix(hex, 16)
                .map_err(|err| TxlineError::proof_decode(err.to_string()))?;
        }
        Ok(Self(bytes))
    }
}

impl fmt::Debug for Hash32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash32(0x")?;
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        write!(f, ")")
    }
}

impl Serialize for Hash32 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Hash32 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(Hash32Visitor)
    }
}

struct Hash32Visitor;

impl<'de> Visitor<'de> for Hash32Visitor {
    type Value = Hash32;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a 32-byte hash as base64, hex, or a byte array")
    }

    fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
    where
        E: DeError,
    {
        Hash32::decode(value).map_err(E::custom)
    }

    fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_str(&value)
    }

    fn visit_bytes<E>(self, value: &[u8]) -> std::result::Result<Self::Value, E>
    where
        E: DeError,
    {
        Hash32::from_bytes(value).map_err(E::custom)
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut bytes = [0u8; 32];
        for (idx, slot) in bytes.iter_mut().enumerate() {
            let Some(byte) = seq.next_element::<u8>()? else {
                return Err(A::Error::custom(format!(
                    "expected 32 bytes, received {idx}"
                )));
            };
            *slot = byte;
        }
        if seq.next_element::<u8>()?.is_some() {
            return Err(A::Error::custom("expected 32 bytes, received more than 32"));
        }
        Ok(Hash32(bytes))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofNode {
    pub hash: Hash32,
    #[serde(rename = "isRightSibling")]
    pub is_right_sibling: bool,
}

impl ProofNode {
    pub fn anchor_hash(&self) -> [u8; 32] {
        self.hash.to_bytes()
    }
}
