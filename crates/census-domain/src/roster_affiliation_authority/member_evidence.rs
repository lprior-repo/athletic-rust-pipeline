mod evidence;
mod facts;
mod interpretation;
mod locator;
mod spans;
mod witness;

pub use evidence::CheckedRosterMemberEvidenceV1;
pub use locator::SourceRowLocator;

use super::{charge, size};
use crate::roster_affiliation_authority::{
    LocallyCheckedRosterIssuanceV1, RosterAuthorityError, RosterAuthorityLimitsV1,
    MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES,
};
use sha2::{Digest, Sha256};

type Result<T, E = RosterAuthorityError> = std::result::Result<T, E>;

pub fn encode_roster_member_witness_v1(
    index: usize,
    preimage: &[u8],
    limits: RosterAuthorityLimitsV1,
) -> Result<Vec<u8>> {
    witness::encode(index, preimage, limits)
}

pub fn check_roster_member_witness_v1(
    original: &LocallyCheckedRosterIssuanceV1,
    interpretation: &[u8],
    witness: &[u8],
    limits: RosterAuthorityLimitsV1,
) -> Result<CheckedRosterMemberEvidenceV1> {
    admit(interpretation, witness, limits)?;
    verify_interpretation(original, interpretation)?;
    let decoded = witness::decode(witness)?;
    let interpreted = interpretation::decode(interpretation)?;
    let row = interpretation::row_at(&interpreted, decoded.inventory_index)?;
    if !digest_matches(
        decoded.semantic_preimage.get().as_bytes(),
        row.semantic_digest,
    ) {
        return Err(RosterAuthorityError::Invalid(
            "witness semantic digest mismatch",
        ));
    }
    let facts = facts::derive(decoded.semantic_preimage, original, &row)?;
    CheckedRosterMemberEvidenceV1::new(original, &row, decoded.inventory_index, facts)
}

fn admit(interpretation: &[u8], witness: &[u8], limits: RosterAuthorityLimitsV1) -> Result<()> {
    if witness.is_empty() {
        return Err(RosterAuthorityError::Invalid("empty witness"));
    }
    charge(
        "interpretation bytes",
        size(interpretation.len())?,
        size(MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES)?,
    )?;
    charge(
        "witness bytes",
        size(witness.len())?,
        size(limits.record_bytes)?,
    )?;
    let total = size(interpretation.len())?
        .checked_add(size(witness.len())?)
        .ok_or(RosterAuthorityError::Overflow)?;
    charge("member evidence bytes", total, limits.component_bytes)
}

fn digest_matches(bytes: &[u8], expected: &str) -> bool {
    let actual = Sha256::digest(bytes);
    expected.len() == 64
        && expected
            .as_bytes()
            .chunks_exact(2)
            .zip(actual.iter())
            .all(|(pair, actual)| match pair {
                [high, low] => {
                    hex(*high)
                        .and_then(|high| high.checked_mul(16))
                        .and_then(|high| hex(*low).and_then(|low| high.checked_add(low)))
                        == Some(*actual)
                }
                _ => false,
            })
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => byte.checked_sub(b'0'),
        b'a'..=b'f' => byte
            .checked_sub(b'a')
            .and_then(|digit| digit.checked_add(10)),
        _ => None,
    }
}

fn verify_interpretation(
    original: &LocallyCheckedRosterIssuanceV1,
    interpretation: &[u8],
) -> Result<()> {
    let reference = original.capture().interpretation();
    if size(interpretation.len())? != reference.bytes().get() {
        return Err(RosterAuthorityError::Invalid(
            "interpretation length mismatch",
        ));
    }
    if !digest_matches(interpretation, reference.digest().as_str()) {
        return Err(RosterAuthorityError::Invalid(
            "interpretation digest mismatch",
        ));
    }
    Ok(())
}
