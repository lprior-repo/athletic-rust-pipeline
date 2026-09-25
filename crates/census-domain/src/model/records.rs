//! Derived rows the census keeps beside its canonical entities: the retained conflict and review
//! queues, the source-to-canonical joins, what a source observed, the merges it decided, and the
//! measurements of a pass.
//!
//! These are [values](crate::model), not tables. They carry no clock, no store handle and no JSON
//! value, so the domain crate stays pure and the store decides how a row is keyed and durable. Every
//! id here is a deterministic function of the row's own facts, which is what lets the same finding be
//! re-derived on a later run without minting a second case.
//!
//! The rows are grouped by what they are evidence of, one module each: [`source`] holds the §31
//! `source object -> canonical row` join and the enumerated meets, [`observation`] holds what a source
//! published about a school or an athlete before any canonical decision,
//! [`source_observation`] carries that pair as one row a store can key, [`merge`] holds the merges
//! this program decided so one can be reversed, and [`measure`] holds coverage, snapshots and access
//! conditions. The queues this module keeps itself are the two findings a pass acts on: the conflict
//! the merge retained, and the review case the lane asks about.

mod measure;
mod merge;
mod observation;
mod source;
mod source_observation;

pub use measure::{
    AccessBlockKind, CollectionSnapshot, CoverageRow, CoverageScope, SourceAccessCondition,
};
pub use merge::CanonicalMerge;
pub use observation::{SourceAthleteObservation, SourceSchoolObservation};
pub use source::{SourceEntityKind, SourceMeetRef, SourceObjectIdentity};
pub use source_observation::SourceObservation;

use serde::{Deserialize, Serialize};
use super::AthleteCandidateId;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// A conflict the merge kept: two canonical rows a stored key says are the same subject.
///
/// Retained rather than resolved, because resolving it is a decision the evidence does not make — the
/// row exists so an operator or the review lane can act on a subject instead of on a count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedConflict {
    pub id: String,
    /// The family label the published queue prints.
    pub family: String,
    /// The canonical row the finding is about.
    pub subject_id: String,
    /// The subject as a human reads it (name, school, meet).
    pub subject: String,
    /// Why the row is unresolved.
    pub detail: String,
}

impl RetainedConflict {
    /// Mint a conflict row. The id binds family and subject, so one subject yields one row per family
    /// however often the derivation runs.
    pub fn new(
        family: &str,
        subject_id: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        let subject_id = subject_id.into();
        Self {
            id: format!("{family}:{subject_id}"),
            family: family.to_string(),
            subject_id,
            subject: subject.into(),
            detail: detail.into(),
        }
    }
}

/// Where a review case stands.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    /// No decision yet: the case is what the review lane is for.
    #[default]
    Pending,
    /// The lane adjudicated it (for example the identity model returned a verdict the merge applied).
    Resolved,
    /// The lane looked and left it: the evidence does not decide, so the row stays visible.
    Retained,
    /// The finding no longer stands: a later pass derived the subject's rows again and this reading of
    /// them is not among the findings it reached, so no decision is owed. A decision already recorded
    /// is never moved here — the verdict is history about the evidence of its time — and the case that
    /// replaced this one carries the state this one was left in.
    Superseded,
}

/// The revision of the review policy a case was minted under.
///
/// A case id binds the revision, so bumping this re-asks every retained finding under the rules as
/// they stand now, instead of letting a decision taken under rules this program no longer applies be
/// read as a decision taken under these.
pub const REVIEW_POLICY_REVISION: u32 = 1;

/// What one case's evidence is, normalized into the digest its id carries.
///
/// A case is a question about a package of evidence, so the id binds the package: each statement the
/// finding rests on is folded to one fact, and the digest is over those facts. A finding re-derived
/// from the same statements — listed in another order, cased or spaced another way — mints the same
/// case, so a repeated derivation never asks the model about the same package twice; a fact that
/// changed at all is new evidence, which mints a new case rather than borrowing an answer given about
/// evidence the model never saw.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CaseEvidence {
    /// One normalized fact per statement the finding rests on.
    facts: BTreeSet<String>,
}

impl CaseEvidence {
    /// The evidence of one finding, as its own statements give it.
    pub fn of<'a>(facts: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            facts: facts.into_iter().map(normalize_fact).collect(),
        }
    }

    /// The digest the case id carries: the first 64 bits of the facts' hash, as hex.
    pub fn digest(&self) -> String {
        let mut hasher = Sha256::new();
        for fact in &self.facts {
            hasher.update(fact.as_bytes());
            hasher.update([0x1f]);
        }
        let mut digest = String::with_capacity(16);
        // SHA-256 always yields 32 bytes; `take(8)` keeps the 64-bit identity the canonical ids use.
        for byte in hasher.finalize().iter().take(8) {
            digest.push_str(&format!("{byte:02x}"));
        }
        digest
    }
}

/// One evidence fact as the digest reads it: ASCII-case-folded, tokens free of surrounding
/// punctuation, whitespace collapsed, tokens sorted.
///
/// Sorting the tokens is what makes the digest a bag of facts rather than a sentence: a writer that
/// lists the same facts in another order — the ids of a retained group, say — re-mints no case. The
/// surrounding punctuation has to go with it, because a list renders as `ath_a, ath_b` and permuting
/// it moves the comma onto another token; a token that is nothing but punctuation carries no fact and
/// is dropped.
fn normalize_fact(fact: &str) -> String {
    let mut tokens: Vec<String> = fact
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_ascii_lowercase()
        })
        .filter(|token| !token.is_empty())
        .collect();
    tokens.sort();
    tokens.join(" ")
}

/// One case the review lane owns (§32).
///
/// The id binds the finding *and its evidence version*: the family and subject say what the case is
/// about, the policy revision and the evidence digest say which reading of that subject it is a
/// question about. The same finding re-derived from the same evidence therefore reuses one case, while
/// evidence that changed mints a new one — the earlier case keeps the state it was left in, and the
/// verdicts recorded under its id stay readable beside it, so a decision is never read as a decision
/// about evidence it never saw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCase {
    pub id: String,
    pub family: String,
    pub subject_id: String,
    pub subject: String,
    pub detail: String,
    pub state: ReviewState,
    /// Raw candidate members named by the finding; absent on legacy JSON and hydrated by indexing.
    #[serde(default)]
    pub member_ids: Vec<AthleteCandidateId>,
}

impl ReviewCase {
    /// Mint a pending case for one finding, under the current policy revision.
    ///
    /// The evidence is what the finding states about itself: the subject line and the detail the merge
    /// retained. Callers that hold more of it — the observed rows, the provider ids — state it in the
    /// detail, because that text is what the id binds.
    pub fn pending(
        family: &str,
        subject_id: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        let subject_id = subject_id.into();
        let subject = subject.into();
        let detail = detail.into();
        let evidence = CaseEvidence::of([subject.as_str(), detail.as_str()]);
        Self {
            id: format!(
                "{family}:{subject_id}:p{REVIEW_POLICY_REVISION}:{}",
                evidence.digest()
            ),
            family: family.to_string(),
            subject_id,
            subject,
            detail,
            state: ReviewState::Pending,
            member_ids: Vec::new(),
        }
    }

    /// Mint one finding's case in the state its family starts in.
    ///
    /// A family starts `Pending` when outside evidence would settle its finding and no decision is
    /// recorded yet: a jurisdiction no source placed and an identity the rows disagree about are
    /// questions the lane, the operator or a sweep answers, and until one does the case is open work.
    ///
    /// A family starts `Retained` when the census's own rules already decided it, which is what
    /// [`Self::decided_by_its_own_rules`] asks: a cohort claim published without evidence that raises
    /// it to the high bar. Minting those `Retained` keeps the row visible in the workbook's queues,
    /// where a reader goes to see it, without counting them as decisions nothing can take: the state
    /// is the one the lane itself leaves a finding it cannot decide.
    pub fn minted(
        family: &str,
        subject_id: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        let mut case = Self::pending(family, subject_id, subject, detail);
        if Self::decided_by_its_own_rules(family) {
            case.state = ReviewState::Retained;
        }
        case
    }

    /// Whether a family's finding is decided by the census's own rules rather than by a later answer.
    ///
    /// One kind qualifies: the evidence rule decides a cohort claim — `identity_confidence` is derived
    /// from the row's observations, and a row without one that agrees is published at the bar its
    /// evidence supports, which is the answer the finding asks for. Everything else in the queues is a
    /// question this store cannot answer for itself.
    pub fn decided_by_its_own_rules(family: &str) -> bool {
        COHORT_DECISION_FAMILIES.contains(&family)
    }
}

/// The family names a retained finding carries, as they are stored in the `family` column.
///
/// They live with the record rather than with the sheet or the lane that renders them, because a
/// reader that matches a name it spelled itself stops matching the moment a writer renames one:
/// the identity lane would quietly ask about no cases, and a seal would count cohort decisions that
/// no longer arrive under the name it looks for. One definition, every reader.
pub const COHORT_EVIDENCE_FAMILY: &str = "Class-of-2027 cohort evidence";
pub const ATHLETE_IDENTITY_FAMILY: &str = "Athlete identity";
pub const SCHOOL_IDENTITY_FAMILY: &str = "School identity";
pub const COHORT_UNVERIFIED_FAMILY: &str = "Class-of-2027 cohort unverified";
pub const COHORT_IDENTITY_CONFIDENCE_FAMILY: &str = "Class-of-2027 identity confidence";

/// The families whose finding is a cohort claim the census publishes without evidence that raises it
/// to the high bar: an athlete in the cohort with no grade observation retained, and one whose
/// observations disagree with the class the row is published under.
///
/// Both are decided by the rule the row is read through — [`CanonicalAthlete::derived_identity_confidence`]
/// derives the confidence from the observations, and the row is published at the bar its evidence
/// supports — so their cases start terminal. The acquisition that
/// would change the answer (a source that carries the grade level) removes the finding itself, and
/// the pass that derives it away closes the case as superseded.
///
/// Kept here, with the family names, because two readers count these cases rather than render them:
/// the seal's cohort item and the mint rule that decides what that item can ever see.
///
/// [`CanonicalAthlete::derived_identity_confidence`]: crate::model::CanonicalAthlete::derived_identity_confidence
pub const COHORT_DECISION_FAMILIES: [&str; 2] =
    [COHORT_UNVERIFIED_FAMILY, COHORT_IDENTITY_CONFIDENCE_FAMILY];
pub const UNRESOLVED_VENUE_FAMILY: &str = "Meet venue unresolved";
pub const UNRESOLVED_SCHOOL_FAMILY: &str = "School jurisdiction unresolved";
/// A school whose coach rows disagree about the address to publish.
pub const CONTACT_CONFLICT_FAMILY: &str = "Recruiting contact conflict";

#[cfg(test)]
#[path = "records_tests.rs"]
mod tests;
