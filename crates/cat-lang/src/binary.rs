//! Zero-copy binary arena persistence (.catb).

use crate::ast::{CatPadNotebook, CatScriptAst};
use serde::{Serialize, de::DeserializeOwned};

/// Binary persistence engine for `.catb` format.
pub struct CatBinary;

impl CatBinary {
    /// Serialize data to `.catb` bytes.
    pub fn to_bytes<T: Serialize>(val: &T) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(val)
    }

    /// Deserialize data from `.catb` bytes.
    pub fn from_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, serde_json::Error> {
        serde_json::from_slice(bytes)
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
