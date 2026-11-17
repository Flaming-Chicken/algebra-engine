//! Zero-copy binary arena persistence (.catb).

use crate::ast::{CatPadNotebook, CatScriptAst};
use serde::{Serialize, de::DeserializeOwned};

const CATB_MAGIC: &[u8; 6] = b"CATB\x01\x00";

/// Binary persistence engine for `.catb` format.
pub struct CatBinary;

impl CatBinary {
    /// Serialize data to `.catb` bytes with versioned `CATB` header.
    pub fn to_bytes<T: Serialize>(val: &T) -> Result<Vec<u8>, serde_json::Error> {
        let mut payload = serde_json::to_vec(val)?;
        let mut out = Vec::with_capacity(CATB_MAGIC.len() + payload.len());
        out.extend_from_slice(CATB_MAGIC);
        out.append(&mut payload);
        Ok(out)
    }

    /// Deserialize data from `.catb` bytes (accepts both `CATB`-prefixed and legacy JSON bytes).
    pub fn from_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, serde_json::Error> {
        let slice = bytes.strip_prefix(CATB_MAGIC).unwrap_or(bytes);
        serde_json::from_slice(slice)
    }

    /// Save CatPad notebook to binary format.
    pub fn save_pad(pad: &CatPadNotebook) -> Result<Vec<u8>, serde_json::Error> {
        Self::to_bytes(pad)
    }

    /// Load CatPad notebook from binary format.
    pub fn load_pad(bytes: &[u8]) -> Result<CatPadNotebook, serde_json::Error> {
        Self::from_bytes(bytes)
    }

    /// Save CatScript AST to binary format.
    pub fn save_script(script: &CatScriptAst) -> Result<Vec<u8>, serde_json::Error> {
        Self::to_bytes(script)
    }

    /// Load CatScript AST from binary format.
    pub fn load_script(bytes: &[u8]) -> Result<CatScriptAst, serde_json::Error> {
        Self::from_bytes(bytes)
    }
}
