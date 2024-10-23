//! Helpers for constructing eip4844 blobs.
//!
//! ref: https://github.com/ethereum/consensus-specs/blob/86fb82b221474cc89387fa6436806507b3849d88/specs/deneb/polynomial-commitments.md
// ref: https://github.com/paradigmxyz/reth/blob/bc43613be35f316304f7ce4a2225855ac26b5923/crates/primitives/src/transaction/sidecar.rs#L254-L256
//

use alloy::{
    consensus::{EnvKzgSettings, SidecarBuilder, SimpleCoder},
    eips::eip4844::USABLE_BYTES_PER_BLOB,
};
use c_kzg::{BYTES_PER_BLOB, BYTES_PER_FIELD_ELEMENT, FIELD_ELEMENTS_PER_BLOB};

// The first element is used as a header (which contains the length of the data, right padded). The
// first bit of each field is element is 0.
const SIMPLE_CODER_MAX_DATA_PER_BLOB: usize = USABLE_BYTES_PER_BLOB - 32 - FIELD_ELEMENTS_PER_BLOB;

/// Errors for this crate.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// kzg blobs error
    #[error("kzg blobs: {0}")]
    Kzg(#[from] c_kzg::Error),
    /// error generating a kzg proof for a blob
    #[error("error generating blob proof: {0}")]
    ProofGen(c_kzg::Error),
    /// error generating a commitment to a blob
    #[error("error generating blob commitment: {0}")]
    CommitmentGen(c_kzg::Error),
}

/// Blob with it's commitment and proof.
pub struct BlobMeta {
    commitment: c_kzg::KzgCommitment,
    blob: c_kzg::Blob,
    proof: c_kzg::KzgProof,
}

impl std::fmt::Debug for BlobMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlobMeta").finish()
    }
}

/// Eip4844 blob generation
#[derive(Debug)]
pub struct Blobs {
    kzg_settings: EnvKzgSettings,
}

impl Blobs {
    /// Create a new instance of [`Self`].
    pub fn new(kzg_settings: EnvKzgSettings) -> Self {
        Self { kzg_settings }
    }

    /// Given a slice of bytes, convert them into blobs.
    ///
    /// Each blob must contain exactly `c_kzg::BYTES_PER_BLOB` bytes. If the `bytes %
    /// c_kzg::BYTES_PER_BLOB` is non-zero, the data in the last blob will be right padded with
    /// zeros.
    ///
    /// Note the this function computes KZG proofs for each blob. This is likely pretty slow
    /// and thus should be handled appropriately in a tokio runtime.
    pub fn generate_blobs_from_bytes(&self, bytes: &[u8]) -> Result<Vec<BlobMeta>, Error> {
        generate_blobs_from_bytes(bytes, self.kzg_settings.get())
    }
}

fn generate_blobs_from_bytes(
    bytes: &[u8],
    settings: &c_kzg::KzgSettings,
) -> Result<Vec<BlobMeta>, Error> {
    let chunks = bytes.chunks_exact(BYTES_PER_BLOB);
    let remainder = chunks.remainder();

    let capacity = bytes.len() / BYTES_PER_BLOB + 1;
    let mut blobs = Vec::with_capacity(capacity);

    for chunk in chunks {
        dbg!("a");
        let blob_meta = bytes_to_meta(chunk, settings)?;
        blobs.push(blob_meta);
    }

    let x = FIELD_ELEMENTS_PER_BLOB;
    if remainder.len() > 0 {
        // Create a 0 filled array of the exact length
        let mut remainder_padded = [0u8; BYTES_PER_BLOB];
        // and memcopy in the "remainder" bytes to the front, leaving the back zero padded.
        remainder_padded[..remainder.len()].copy_from_slice(remainder);

        let blob_meta = bytes_to_meta(&remainder_padded, settings)?;
        blobs.push(blob_meta);
    }

    Ok(blobs)
}

fn bytes_to_meta(bytes: &[u8], settings: &c_kzg::KzgSettings) -> Result<BlobMeta, Error> {
    let blob = c_kzg::Blob::from_bytes(bytes)?;
    let commitment = c_kzg::KzgCommitment::blob_to_kzg_commitment(&blob, settings)
        .map_err(Error::CommitmentGen)?;
    let proof = c_kzg::KzgProof::compute_blob_kzg_proof(&blob, &commitment.to_bytes(), settings)
        .map_err(Error::ProofGen)?;

    Ok(BlobMeta { blob, commitment, proof })
}

#[cfg(test)]
mod test {
    use crate::SIMPLE_CODER_MAX_DATA_PER_BLOB;
    use alloy::{
        consensus::{SidecarBuilder, SidecarCoder, SimpleCoder},
        eips::eip4844::USABLE_BYTES_PER_BLOB,
    };
    use c_kzg::{BYTES_PER_BLOB, BYTES_PER_FIELD_ELEMENT, FIELD_ELEMENTS_PER_BLOB};

    use crate::Blobs;

    fn four_blob_vec() -> Vec<u8> {
        let len = SIMPLE_CODER_MAX_DATA_PER_BLOB * 3 + 256;

        (0..len).map(|n| n as u8 % u8::MAX).collect()
    }

    fn seven_blob_vec() -> Vec<u8> {
        let len = SIMPLE_CODER_MAX_DATA_PER_BLOB * 6 + 256;

        (0..len).map(|n| n as u8 % u8::MAX).collect()
    }

    #[test]
    fn simple_coder() {
        let four_blobs = vec![four_blob_vec()];
        let builder = four_blobs.iter().collect::<SidecarBuilder<SimpleCoder>>();
        let x = USABLE_BYTES_PER_BLOB;

        let four = builder.build().unwrap();
        assert_eq!(four.blobs.len(), 4);

        let raw_data = vec![seven_blob_vec()];
        let builder = seven_blobs.iter().collect::<SidecarBuilder<SimpleCoder>>();
        let built = builder.build().unwrap();
        assert_eq!(built.blobs.len(), 7);

        let c = SimpleCoder {};
        let data = c.decode_all(built.blobs).unwrap();
    }

    #[test]
    fn check_understanding() {
        assert_eq!(FIELD_ELEMENTS_PER_BLOB, 4096);
        assert_eq!(BYTES_PER_FIELD_ELEMENT, 32);
    }
}
