//! Helpers for constructing eip4844 blobs and commitments.
//!
//! ref: https://github.com/paradigmxyz/reth/blob/bc43613be35f316304f7ce4a2225855ac26b5923/crates/primitives/src/transaction/sidecar.rs#L254-L256

use alloy::consensus::EnvKzgSettings;
use c_kzg::BYTES_PER_BLOB;

/// Errors for this crate.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("kzg blobs: {0}")]
    Kzg(#[from] c_kzg::Error),
    #[error("error generating blob proof: {0}")]
    ProofGen(c_kzg::Error),
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
        let blob_meta = bytes_to_meta(chunk, settings)?;
        blobs.push(blob_meta);
    }

    if remainder.len() > 0 {
        // Create a 0 filled array of the exact length
        let mut remainder_padded = [0u8; BYTES_PER_BLOB];
        // and memcopy in the the remainder bytes to the front.
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
