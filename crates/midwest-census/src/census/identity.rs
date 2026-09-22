//! Deterministic workflow identities (objective §8).
//!
//! Every durable unit of work — one jurisdiction's census, one source sweep, one meet, one athlete,
//! one identity review — is addressed by a string that is a pure function of the values that define
//! the work: jurisdiction, season, revision, source object id. A retry reuses it.
//!
//! Nothing here depends on a timestamp, a counter, a random value or a run id. That is the whole
//! point: an identity that changed after a failed HTTP call would turn one logical job into an
//! unbounded family of jobs, and the journal would fill with duplicate work that no operator can
//! distinguish from real progress. `{revision}` is the one escape hatch — an operator bumps it to
//! invalidate completed work deliberately, which is a decision, not an accident.
//!
//! # Bounds
//!
//! Restate addresses objects and workflows by a UTF-8 key, so an identity is a key. External source
//! ids are unbounded strings: a meet id from a timing provider can be any length and can contain the
//! `:` separator this module joins fields with. A dynamic part that is empty, longer than
//! [`MAX_PART_BYTES`], or carrying a byte outside `[A-Za-z0-9._-]` is therefore replaced by its
//! digest. That keeps every identity inside [`MAX_IDENTITY_BYTES`], keeps the field structure
//! unambiguous, and costs nothing: the full provider value lives in the store row the workflow reads.

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use sha2::{Digest, Sha256};
use std::fmt;

/// Longest identity this crate mints, in bytes.
///
/// Restate's own ceiling is the server's, not the SDK's; this bound is ours and it exists so an
/// identity cannot grow with a provider's id space and stays readable in `restate` CLI output and
/// server logs. The longest pattern — `source-sweep:<source>:<jurisdiction>:<season>:<revision>` —
/// fits with room for a 32-byte source slug.
pub const MAX_IDENTITY_BYTES: usize = 64;

/// Longest dynamic part that rides verbatim. Longer parts are digested, not truncated, so two
/// different provider ids can never collapse onto one identity.
const MAX_PART_BYTES: usize = 32;

/// Bytes a digested part keeps. Eight bytes of SHA-256 is the same identity width the domain's
/// `Id::mint` uses for entity ids, so the collision argument is already the repo's own.
const DIGEST_BYTES: usize = 8;

/// A pipeline revision. Bumping it invalidates completed work for that identity on purpose: the
/// operator is saying the acquisition or normalization changed enough that a cached completion is no
/// longer evidence about the current pipeline.
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

/// A workflow address: `<pattern>:<field>[:<field>…]`, built only by the constructors below.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct WorkflowIdentity(String);

impl WorkflowIdentity {
    /// `national:<season>:<revision>` — the root run that fans out one jurisdiction census per
    /// state. One national run per season and revision: restarting it resumes that run instead of
    /// starting a second fan-out over the same work.
    pub fn national(season: SchoolYear, revision: Revision) -> Self {
        Self::join("national", &[&season.short(), &revision.to_string()])
    }

    /// `jurisdiction:<state>:<season>:<revision>` — one jurisdiction's census for one season.
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

    /// `source-sweep:<source>:<state>:<season>:<revision>` — one source's sweep of one jurisdiction.
    pub fn source_sweep(
        source: &str,
        jurisdiction: UsJurisdiction,
        season: SchoolYear,
        revision: Revision,
    ) -> Self {
        Self::join(
            "source-sweep",
            &[
                source,
                jurisdiction.code(),
                &season.short(),
                &revision.to_string(),
            ],
        )
    }

    /// `meet:<source>:<source_meet_id>:<revision>` — one meet's acquisition.
    pub fn meet(source: &str, source_meet_id: &str, revision: Revision) -> Self {
        Self::join("meet", &[source, source_meet_id, &revision.to_string()])
    }

    /// `athlete:<source>:<source_athlete_id>:<revision>` — one athlete's enrichment.
    pub fn athlete(source: &str, source_athlete_id: &str, revision: Revision) -> Self {
        Self::join(
            "athlete",
            &[source, source_athlete_id, &revision.to_string()],
        )
    }

    /// `school:<source>:<source_school_id>:<revision>` — one school's enrichment.
    pub fn school(source: &str, source_school_id: &str, revision: Revision) -> Self {
        Self::join("school", &[source, source_school_id, &revision.to_string()])
    }

    /// `review:<evidence_digest>:<policy_revision>` — one identity review under one policy revision.
    ///
    /// Keyed by the evidence digest, not by a candidate pair: identical evidence packages reuse the
    /// prior review, which is what keeps AI inference a scarce resource (§32).
    pub fn review(evidence_digest: &str, policy_revision: Revision) -> Self {
        Self::join("review", &[evidence_digest, &policy_revision.to_string()])
    }

    /// The wire form: the string Restate receives as the object key or workflow id.
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

/// Append one field, digesting anything that would make the identity ambiguous or unbounded.
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

/// One lowercase hex digit for a nibble (`0..=15` by construction; wider inputs saturate rather
/// than panic, so the loop above has no failing path).
fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0'.saturating_add(nibble)),
        _ => char::from(b'a'.saturating_add(nibble.saturating_sub(10))),
    }
}

/// The bytes a field may carry verbatim: the URL-safe identifier alphabet, minus `:` so the field
/// separator cannot be forged from inside a field.
fn is_part_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' || byte == b'.'
}

#[cfg(test)]
mod tests;
