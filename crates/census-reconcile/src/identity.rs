
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use sha2::{Digest, Sha256};
use std::fmt;

pub const MAX_IDENTITY_BYTES: usize = 64;

const MAX_PART_BYTES: usize = 32;

const DIGEST_BYTES: usize = 8;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct Revision(pub u32);

impl Revision {
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct WorkflowIdentity(String);

impl WorkflowIdentity {
    pub fn national(
        season: SchoolYear,
        revision: Revision,
        jurisdictions: &[UsJurisdiction],
    ) -> Self {
        Self::join(
            "national",
            &[
                &season.short(),
                &scope_digest(jurisdictions),
                &revision.to_string(),
            ],
        )
    }

    pub fn jurisdiction(
        jurisdiction: UsJurisdiction,
        season: SchoolYear,
        revision: Revision,
    ) -> Self {
        Self::join(
            "jurisdiction",
            &[jurisdiction.code(), &season.short(), &revision.to_string()],
        )
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn join(pattern: &str, fields: &[&str]) -> Self {
        let mut value = String::with_capacity(MAX_IDENTITY_BYTES);
        value.push_str(pattern);
        for field in fields {
            value.push(':');
            push_bounded(&mut value, field);
        }
        Self(value)
    }
}

impl fmt::Display for WorkflowIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn admitted_scope(jurisdictions: &[UsJurisdiction]) -> Vec<UsJurisdiction> {
    if jurisdictions.is_empty() {
        UsJurisdiction::CENSUS_SCOPE.to_vec()
    } else {
        jurisdictions.to_vec()
    }
}

fn scope_digest(jurisdictions: &[UsJurisdiction]) -> String {
    let admitted = admitted_scope(jurisdictions);
    let mut ordered: Vec<UsJurisdiction> = UsJurisdiction::CENSUS_SCOPE
        .iter()
        .copied()
        .filter(|state| admitted.contains(state))
        .collect();
    let mut outside: Vec<UsJurisdiction> = admitted
        .iter()
        .copied()
        .filter(|state| !ordered.contains(state))
        .collect();
    outside.sort_by_key(|state| state.code());
    outside.dedup();
    ordered.extend(outside);

    let mut hasher = Sha256::new();
    for state in &ordered {
        hasher.update(state.code().as_bytes());
        hasher.update(b"\n");
    }
    let sum = hasher.finalize();
    let mut digest = String::with_capacity(DIGEST_BYTES.saturating_mul(2));
    for byte in sum.iter().take(DIGEST_BYTES) {
        digest.push(hex_digit(byte >> 4));
        digest.push(hex_digit(byte & 0x0f));
    }
    digest
}

fn push_bounded(out: &mut String, field: &str) {
    if field.is_empty() || field.len() > MAX_PART_BYTES || !field.bytes().all(is_part_byte) {
        out.push('h');
        let mut hasher = Sha256::new();
        hasher.update(field.as_bytes());
        let sum = hasher.finalize();
        for byte in sum.iter().take(DIGEST_BYTES) {
            out.push(hex_digit(byte >> 4));
            out.push(hex_digit(byte & 0x0f));
        }
        return;
    }
    out.push_str(field);
}

fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0'.saturating_add(nibble)),
        _ => char::from(b'a'.saturating_add(nibble.saturating_sub(10))),
    }
}

fn is_part_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' || byte == b'.'
}

#[cfg(test)]
mod tests;
