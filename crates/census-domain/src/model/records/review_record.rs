//! Queue records: conflicts the merge retained and review cases the lane acts on.
//!
//! These rows answer "what is unresolved and needs attention". They are evidence, not tables:
//! they carry no clock, no store handle and no JSON value, so the domain crate stays pure and the
//! store decides how a row is keyed and durable.

use serde::{Deserialize, Serialize};

use crate::model::AthleteCandidateId;

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
///
/// Revision 2 replaced the bag-of-tokens evidence digest with structured facts: statements keep their
/// token order (so a swapped attribution is different evidence) and signs survive normalization (so
/// `-1.4` is not `1.4`), and a finding's candidate membership is hashed with it. Every id minted under
/// revision 1 is therefore retired rather than reinterpreted — the verdicts recorded under those ids
/// stay readable beside the new ones, which is the whole point of binding the revision into the id.
pub const REVIEW_POLICY_REVISION: u32 = 2;

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
/// Cohort confidence is derived by [`CanonicalAthlete::derived_cohort_confidence`], separately from
/// identity acceptance. These cases describe missing or contradictory cohort evidence; acquisition
/// can remove a finding by retaining the missing published grade.
///
/// Kept here, with the family names, because two readers count these cases rather than render them:
/// the seal's cohort item and the mint rule that decides what that item can ever see.
///
/// [`CanonicalAthlete::derived_cohort_confidence`]: crate::model::CanonicalAthlete::derived_cohort_confidence
pub const COHORT_DECISION_FAMILIES: [&str; 2] =
    [COHORT_UNVERIFIED_FAMILY, COHORT_IDENTITY_CONFIDENCE_FAMILY];
pub const UNRESOLVED_VENUE_FAMILY: &str = "Meet venue unresolved";
pub const UNRESOLVED_SCHOOL_FAMILY: &str = "School jurisdiction unresolved";
/// A school whose coach rows disagree about the address to publish.
pub const CONTACT_CONFLICT_FAMILY: &str = "Recruiting contact conflict";

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
    /// retained. A caller that holds more of it — the observed rows, the provider ids — mints through
    /// [`Self::pending_with_evidence`] instead and puts that structure in the package, because the
    /// package is what the id binds.
    pub fn pending(
        family: &str,
        subject_id: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        let subject = subject.into();
        let detail = detail.into();
        let evidence = super::evidence::CaseEvidence::of([subject.as_str(), detail.as_str()]);
        Self::pending_with_evidence(family, subject_id, subject, detail, evidence)
    }

    /// Mint a displayable case while binding its id to the evidence package it rests on.
    pub fn pending_with_evidence(
        family: &str,
        subject_id: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
        evidence: super::evidence::CaseEvidence,
    ) -> Self {
        let subject_id = subject_id.into();
        Self {
            id: format!(
                "{family}:{subject_id}:p{REVIEW_POLICY_REVISION}:{}",
                evidence.digest()
            ),
            family: family.to_string(),
            subject_id,
            subject: subject.into(),
            detail: detail.into(),
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
