//! The identity-review lane's vocabulary (§32–§35): what the lane is asked, and what it is allowed
//! to answer.
//!
//! The lane exists because a merge cannot decide identity from a stored key alone: two canonical
//! rows can agree on a name and disagree on a school, a state or a grade, and the evidence does not
//! pick one. `ReviewCase` records the finding; these types are the question and the answer.
//!
//! Two rules make the answer safe to apply:
//!
//! * A packet names the case ids it asks about, and the reader rejects any verdict naming a case the
//!   packet did not ask about — so a model cannot invent work.
//! * A verdict carries a kind and a rationale but no authority; the merge applies it, and
//!   `InsufficientEvidence` leaves the case visible rather than resolving it by default.

use super::AthleteCandidateId;
use serde::{Deserialize, Serialize};

/// One fact the lane is shown about a case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewEvidenceFact {
    /// The source namespace the fact came from (`milesplit_athlete`, `timer_meet:wayzata`, …).
    pub source: String,
    /// The field the fact is about (`name`, `school`, `state`, `grad_year`, `observed_grade`, …).
    pub field: String,
    pub value: String,
}

impl ReviewEvidenceFact {
    /// Mint one fact.
    pub fn new(
        source: impl Into<String>,
        field: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            field: field.into(),
            value: value.into(),
        }
    }
}

/// One case inside a packet, as the model reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCaseFact {
    pub case_id: String,
    pub family: String,
    pub detail: String,
}

/// Everything the lane is told about one retained subject: the subject, the cases, and the evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewPacket {
    /// The store's id for the subject the cases are about — an athlete, a school or a meet.
    pub subject_id: String,
    /// The subject as a human reads it.
    pub subject: String,
    pub cases: Vec<ReviewCaseFact>,
    pub evidence: Vec<ReviewEvidenceFact>,
}

impl ReviewPacket {
    /// Mint an empty packet for one subject.
    pub fn new(subject_id: impl Into<String>, subject: impl Into<String>) -> Self {
        Self {
            subject_id: subject_id.into(),
            subject: subject.into(),
            cases: Vec::new(),
            evidence: Vec::new(),
        }
    }

    /// Ask about one case.
    pub fn with_case(mut self, case: ReviewCaseFact) -> Self {
        self.cases.push(case);
        self
    }

    /// Show one fact.
    pub fn with_evidence(mut self, fact: ReviewEvidenceFact) -> Self {
        self.evidence.push(fact);
        self
    }

    /// The case ids this packet asks about: the only ids a verdict for it may name.
    pub fn case_ids(&self) -> Vec<String> {
        self.cases.iter().map(|case| case.case_id.clone()).collect()
    }
}

/// What the lane decided about one case.
///
/// The retained families ask for different things — a school's jurisdiction, a meet's venue, whether
/// a cohort claim is corroborated — so the kind records *whether an answer was offered*, and the
/// answer itself travels as a field name and a value. That keeps the vocabulary shared across
/// families without pretending a venue and a state are the same sort of judgment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdictKind {
    /// The lane proposes a value for the field the case left unresolved.
    ValueProposed,
    /// The evidence does not decide; the case stays visible for an operator.
    InsufficientEvidence,
}

impl ReviewVerdictKind {
    /// The slug the store keys a verdict by and the report prints.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::ValueProposed => "value_proposed",
            Self::InsufficientEvidence => "insufficient_evidence",
        }
    }

    /// Parse the slug a model returned, in the spellings models drift between.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "value_proposed" | "value proposed" | "valueproposed" | "proposed" => {
                Some(Self::ValueProposed)
            }
            "insufficient_evidence" | "insufficient evidence" | "insufficientevidence" => {
                Some(Self::InsufficientEvidence)
            }
            _ => None,
        }
    }
}

/// One verdict, with the model's own confidence and the reason it gave.
///
/// A proposal carries the field it answers (`state`, `venue`) and the value it offers. Nothing here
/// is authoritative: the caller validates the value against the store's own evidence before any row
/// is written, which is why the lane can ask a small local model at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewVerdict {
    pub case_id: String,
    pub kind: ReviewVerdictKind,
    /// The field the proposal answers; `None` when the lane offered no value.
    pub field: Option<String>,
    /// The proposed value; `None` when the lane offered no value.
    pub value: Option<String>,
    /// 0–100 as the model reported it. Evidence, not a key: the reader clamps rather than trusting it.
    pub confidence: u8,
    pub rationale: String,
}

/// One athlete's or subject's verdicts, as the model returns them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerdictBatch {
    pub subject_id: String,
    pub verdicts: Vec<ReviewVerdict>,
}

impl VerdictBatch {
    /// Clamp and filter one batch against the packet it answers.
    ///
    /// This is the only place a model's answer becomes admissible:
    ///
    /// * verdicts for cases the packet did not ask about are dropped — a model cannot invent work;
    /// * a second verdict for a case already answered is dropped — the batch is read in order and the
    ///   model is asked for one verdict per case;
    /// * a `ValueProposed` verdict without both a field and a non-empty value is demoted to
    ///   `InsufficientEvidence` — a proposal that proposes nothing is not a proposal;
    /// * a confidence above 100 is clamped to the field's range.
    ///
    /// Returns the admissible verdicts and how many were dropped, so the caller can report the
    /// difference instead of hiding a model that ignored its instructions.
    pub fn sanitize(self, packet: &ReviewPacket) -> (Vec<ReviewVerdict>, usize) {
        let asked = packet.case_ids();
        let mut admitted: Vec<ReviewVerdict> = Vec::with_capacity(self.verdicts.len());
        let mut dropped = 0_usize;
        for mut verdict in self.verdicts {
            if !asked.contains(&verdict.case_id)
                || admitted.iter().any(|kept| kept.case_id == verdict.case_id)
            {
                dropped = dropped.saturating_add(1);
                continue;
            }
            verdict.confidence = verdict.confidence.min(100);
            if verdict.kind == ReviewVerdictKind::ValueProposed && !verdict.proposes_a_value() {
                verdict.kind = ReviewVerdictKind::InsufficientEvidence;
                verdict.field = None;
                verdict.value = None;
            }
            admitted.push(verdict);
        }
        (admitted, dropped)
    }
}

impl ReviewVerdict {
    /// Whether this verdict carries a usable proposal: both halves present and the value non-empty.
    pub fn proposes_a_value(&self) -> bool {
        let named = |slot: &Option<String>| {
            slot.as_deref()
                .map(str::trim)
                .is_some_and(|text| !text.is_empty())
        };
        named(&self.field) && named(&self.value)
    }
}

/// One recorded adjudication, keyed by the case it answers: the durable row the review lane writes and
/// an operator reads back.
///
/// The record keeps the model's own words (`rationale`), the value it proposed, and whether validation
/// admitted it, so a refused proposal is as auditable as an accepted one — a case closed with a
/// refusal is a case a later pass can revisit with better evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewVerdictRecord {
    /// The case this verdict answers; also the row's id, so re-asking overwrites rather than appends.
    pub id: String,
    pub case_id: String,
    pub subject_id: String,
    pub family: String,
    /// The verdict's slug: `value_proposed` or `insufficient_evidence`.
    pub kind: String,
    /// The field the proposal answered, empty when none was offered.
    pub field: String,
    /// The value proposed — or the value validation admitted, when it differs in case or spacing.
    pub value: String,
    /// Whether validation admitted the proposal.
    pub accepted: bool,
    pub confidence: u8,
    pub rationale: String,
    /// The model that answered.
    pub reviewer: String,
    /// The lane pass's date.
    pub observed_at: String,
    /// Raw candidate members captured with the current review case.
    #[serde(default)]
    pub member_ids: Vec<AthleteCandidateId>,
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;
