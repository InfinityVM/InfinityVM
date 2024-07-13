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
    pub(crate) fn new(vm_type: u8, verifying_key: &[u8]) -> Self {
        let mut inner: [u8; 32] = Sha256::digest(verifying_key).into();
        inner[0] = vm_type;

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

reth_db::tables! {
    /// Stores risc0 Elf files
    table Risc0ElfTable<Key = ElfKey, Value = Vec<u8>>;

    /// Stores sp1 Elf files
    table Sp1ElfTable<Key = ElfKey, Value = Vec<u8>>;
}
