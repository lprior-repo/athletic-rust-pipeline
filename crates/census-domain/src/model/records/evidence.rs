//! Evidence hashing: case evidence, facts, and statement normalization.
//!
//! These types let a review case id bind the package of evidence it rests on, so the same evidence
//! mints the same case id and changed evidence mints a new one.

use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use crate::model::AthleteCandidateId;

/// What one case's evidence is, normalized into the digest its id carries.
///
/// A case is a question about a package of evidence, so the id binds the package: each statement the
/// finding rests on is folded to one fact, plus one fact per labelled set of candidates the finding
/// names, and the digest is over those facts. A finding re-derived from the same package mints the
/// same case, so a repeated derivation never asks the model about the same package twice; a fact that
/// changed at all is new evidence, which mints a new case rather than borrowing an answer given about
/// evidence the model never saw.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CaseEvidence {
    /// One fact per statement the finding rests on, plus one per labelled member set.
    facts: BTreeSet<EvidenceFact>,
}

impl CaseEvidence {
    /// The evidence of one finding, as its own statements give it.
    pub fn of<'a>(statements: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            facts: statements
                .into_iter()
                .map(|statement| EvidenceFact::Statement(normalize_statement(statement)))
                .collect(),
        }
    }

    /// The same evidence, plus the labelled set of candidates the finding is about.
    ///
    /// Membership is evidence, so it belongs in the digest: a finding resting on three rows is not
    /// the finding resting on two, and an id that ignored the difference would let a verdict taken
    /// about the smaller group answer the larger one. Passing the members here — rather than
    /// assigning them to the case afterwards — is what puts them under the hash.
    pub fn with_members(
        mut self,
        label: &str,
        ids: impl IntoIterator<Item = AthleteCandidateId>,
    ) -> Self {
        let ids: BTreeSet<AthleteCandidateId> = ids.into_iter().collect();
        if !ids.is_empty() {
            self.facts.insert(EvidenceFact::Members {
                label: label.to_string(),
                ids,
            });
        }
        self
    }

    /// The digest the case id carries: the first 64 bits of the facts' hash, as hex.
    pub fn digest(&self) -> String {
        let mut hasher = Sha256::new();
        for fact in &self.facts {
            fact.hash_into(&mut hasher);
        }
        let mut digest = String::with_capacity(16);
        for byte in hasher.finalize().iter().take(8) {
            digest.push_str(&format!("{byte:02x}"));
        }
        digest
    }
}

/// The label a finding's candidate-member set is hashed under.
///
/// One definition, because the label is inside the digest: a caller that spelled it differently would
/// mint a second case for evidence the first both describes and answers.
pub const MEMBER_SET_LABEL: &str = "members";

/// One evidence fact, in the shape its meaning needs.
///
/// The digest cannot tell the two shapes apart from the text alone, and folding every statement into
/// one sorted bag of tokens — the previous reading — is what let a swap of the attribution inside a
/// statement hash the same as the statement it swapped. So a fact says which shape it is:
///
/// * a [`Statement`](Self::Statement) is compared as written, because the order of its tokens *is*
///   part of what it says;
/// * a [`Members`](Self::Members) set is sorted and deduplicated, because a group listed in another
///   order is the same group, and re-asking the model about it would be work with the same answer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceFact {
    /// One statement of the finding, normalized but left in the order it was written.
    Statement(String),
    /// A labelled set of candidates the finding is about, compared as a set.
    Members {
        label: String,
        ids: BTreeSet<AthleteCandidateId>,
    },
}

impl EvidenceFact {
    /// Write this fact into the digest behind a shape tag and separated by `0x1f`, closed by `0x1e`.
    ///
    /// The separators are what stop two shapes from colliding by concatenation: without them a
    /// statement ending in `2027` and one beginning `2027` would produce the same byte run, and the
    /// length of an id list would be indistinguishable from the next fact's opening.
    fn hash_into(&self, hasher: &mut Sha256) {
        match self {
            Self::Statement(text) => {
                hasher.update(b"s\x1f");
                hasher.update(text.as_bytes());
            }
            Self::Members { label, ids } => {
                hasher.update(b"m\x1f");
                hasher.update(label.as_bytes());
                for id in ids {
                    hasher.update(b"\x1f");
                    hasher.update(id.as_str().as_bytes());
                }
            }
        }
        hasher.update([0x1e]);
    }
}

/// One statement as the digest reads it: ASCII-case-folded, tokens cleared of the punctuation that
/// only formatted them, whitespace collapsed — and the token order left exactly as written.
///
/// Order is the whole difference between a bag of words and a statement. `side_a grad_year 2027
/// side_b grad_year 2028` and its attribution swapped carry the same tokens in another order and
/// mean opposite things; hashing the bag minted one case id for both, so a verdict given about one
/// package was read as a verdict about the other. The comma still has to go, because a list renders
/// as `ath_a, ath_b` and the separator attaches itself to whichever token precedes it — but a sign
/// stays, because it is part of the value: `-1.4` m/s is not `1.4` m/s.
pub fn normalize_statement(statement: &str) -> String {
    let tokens: Vec<String> = statement
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '+' && c != '-')
                .to_ascii_lowercase()
        })
        .filter(|token| !token.is_empty())
        .collect();
    tokens.join(" ")
}
