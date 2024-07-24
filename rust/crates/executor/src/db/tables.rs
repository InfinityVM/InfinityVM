use reth_codecs::reth_codec;
use reth_db::{
    table::{Decode, Encode},
    tables, DatabaseError, TableType, TableViewer,
};
use sha2::{Digest, Sha256};
use std::fmt;
/// Key to a table storing ELFs. The first byte of the key is the vm type
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct ElfKey(pub [u8; 32]);

impl ElfKey {
    /// New [Self]
    pub(crate) fn new(verifying_key: &[u8]) -> Self {
        let inner: [u8; 32] = Sha256::digest(verifying_key).into();

        Self(inner)
    }
}

impl Encode for ElfKey {
    type Encoded = [u8; 32];

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decode for ElfKey {
    fn decode<B: AsRef<[u8]>>(value: B) -> Result<Self, DatabaseError> {
        let inner: [u8; 32] = value.as_ref().try_into().map_err(|_| DatabaseError::Decode)?;

        Ok(Self(inner))
    }
}

/// Storage format for elf files
#[derive(Debug, bitcode::Encode, bitcode::Decode, serde::Serialize, serde::Deserialize)]
#[reth_codec]
pub struct ElfWithMeta {
    /// The type of vm
    pub vm_type: u8,
    /// The ELF file
    pub elf: Vec<u8>,
}

impl reth_db::table::Encode for ElfWithMeta {
    type Encoded = Vec<u8>;
    fn encode(self) -> Self::Encoded {
        bitcode::encode(&self)
    }
}

impl reth_db::table::Decode for ElfWithMeta {
    fn decode<B: AsRef<[u8]>>(value: B) -> Result<Self, DatabaseError> {
        bitcode::decode(value.as_ref()).map_err(|_| DatabaseError::Decode)
    }
}

reth_db::tables! {
    /// Stores Elf files
    table ElfTable<Key = ElfKey, Value = ElfWithMeta>;
}
