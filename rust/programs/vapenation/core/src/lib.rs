//! Crate with things we can reuse in the guest program (targets risc-v) and
//! in the host or anywhere else (targets local arch e.g. amd64).

const FOUR_TWENTY: u64 = 420;

/// Metadata for a Vape Nation
#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
pub struct VapeNationMetadata {
    pub nation_id: u64,
    pub phrase: String,
    pub points: u64,
}

/// Compute the ID for a Vape Nation
pub fn compute_nation_id(input: u64) -> u64 {
    (0..input * FOUR_TWENTY).fold(0, |acc, cur| {
        if acc > u64::MAX / 2 {
            acc
        } else {
            let intermediate = (acc + cur) * 3;
            intermediate / 3
        }
    })
}
