use crate::bests::participation::{ensure_assessment, ParticipationSelection};
use crate::bests::{
    ParticipationExclusion, ParticipationExclusionOutcome, ParticipationExclusionReason,
    SelectionOutcome,
};
use crate::export::{
    ExportDataset, ParticipationAssessment, ParticipationDiagnostic, ParticipationOutcome,
    VerifiedParticipationView,
};
use crate::grouping::Admission;
use crate::report::{ReportError, ReportResult};
use serde::{Serialize, Serializer};
use std::path::Path;

use super::verify_sidecars::normalized::rows;

mod documentary;

const FILE: &str = "participation_exclusions.csv";
const HEADER: [&str; 8] = [
    "retained_candidate_id",
    "performance_id",
    "event_id",
    "disposition",
    "reasons",
    "diagnostics",
    "evidence_refs",
    "input_generation",
];

pub(super) fn write(
    directory: &Path,
    dataset: &ExportDataset,
    participation: &VerifiedParticipationView,
    outcome: &SelectionOutcome,
) -> ReportResult<u64> {
    validate(dataset, participation, outcome)?;
    let mut budget = Admission::new("participation exclusion publication");
    super::normalized::write_csv(&directory.join(FILE), &HEADER, |writer| {
        outcome
            .participation_exclusions
            .iter()
            .try_for_each(|value| writer.push(record(value, participation, &mut budget)?.fields()?))
    })
}

pub(super) fn verify(
    directory: &Path,
    dataset: &ExportDataset,
    participation: &VerifiedParticipationView,
    outcome: &SelectionOutcome,
) -> ReportResult<u64> {
    validate(dataset, participation, outcome)?;
    let expected = &outcome.participation_exclusions;
    let mut budget = Admission::new("participation exclusion readback");
    rows::table(
        directory,
        (FILE, &HEADER),
        expected.len(),
        &["performance_id"],
        |found| {
            let id = found.get("performance_id")?;
            let position = expected
                .binary_search_by(|value| value.performance_id().as_str().cmp(id))
                .map_err(|_| defect("participation exclusion references a foreign performance"))?;
            let value = expected.get(position).ok_or_else(|| {
                defect("participation exclusion references a missing expected performance")
            })?;
            verify_record(found, value, participation, &mut budget)
        },
    )
}

fn verify_record(
    found: &rows::Row<'_>,
    value: &ParticipationExclusion,
    participation: &VerifiedParticipationView,
    budget: &mut Admission,
) -> ReportResult<()> {
    let assessment = assessment(value, participation)?;
    let diagnostics: &[_] = assessment.map_or(&[], ParticipationAssessment::diagnostics);
    let evidence_refs: &[_] = assessment.map_or(&[], ParticipationAssessment::evidence_refs);
    let reasons = Reasons(value.reason());
    budget.record(&(
        value.athlete_id().as_str(),
        value.performance_id().as_str(),
        value.event_id().as_str(),
        disposition(value.outcome()),
        &reasons,
        diagnostics,
        evidence_refs,
        participation.input_generation(),
    ))?;
    found.text("retained_candidate_id", value.athlete_id().as_str())?;
    found.text("performance_id", value.performance_id().as_str())?;
    found.text("event_id", value.event_id().as_str())?;
    found.text("disposition", disposition(value.outcome()))?;
    found.text("reasons", encode(&reasons)?)?;
    found.text("diagnostics", encode(&diagnostics)?)?;
    found.text("evidence_refs", encode(&evidence_refs)?)?;
    found.text("input_generation", participation.input_generation())
}

fn validate(
    dataset: &ExportDataset,
    participation: &VerifiedParticipationView,
    outcome: &SelectionOutcome,
) -> ReportResult<()> {
    ParticipationSelection::Selected(participation).ensure_dataset(dataset)?;
    participation.require_documentary_publication()?;
    let mut budget = Admission::new("participation exclusion inventory");
    budget.charge(0, outcome.participation_exclusions.len())?;
    outcome
        .participation_exclusions
        .windows(2)
        .try_for_each(|pair| {
            if let [left, right] = pair {
                if left.performance_id() >= right.performance_id() {
                    return Err(defect(
                        "participation exclusion inventory is duplicate or unordered",
                    ));
                }
            }
            Ok(())
        })?;
    documentary::require_complete(participation, outcome, &mut budget)
}

#[derive(Serialize)]
struct Record<'a> {
    retained_candidate_id: &'a str,
    performance_id: &'a str,
    event_id: &'a str,
    disposition: &'static str,
    reasons: Reasons,
    diagnostics: &'a [ParticipationDiagnostic],
    evidence_refs: &'a [String],
    input_generation: &'a str,
}

impl Record<'_> {
    fn fields(self) -> ReportResult<[String; 8]> {
        Ok([
            self.retained_candidate_id.into(),
            self.performance_id.into(),
            self.event_id.into(),
            self.disposition.into(),
            encode(&self.reasons)?,
            encode(&self.diagnostics)?,
            encode(&self.evidence_refs)?,
            self.input_generation.into(),
        ])
    }
}

fn record<'a>(
    value: &'a ParticipationExclusion,
    participation: &'a VerifiedParticipationView,
    budget: &mut Admission,
) -> ReportResult<Record<'a>> {
    let assessment = assessment(value, participation)?;
    let row = Record {
        retained_candidate_id: value.athlete_id().as_str(),
        performance_id: value.performance_id().as_str(),
        event_id: value.event_id().as_str(),
        disposition: disposition(value.outcome()),
        reasons: Reasons(value.reason()),
        diagnostics: assessment.map_or(&[][..], ParticipationAssessment::diagnostics),
        evidence_refs: assessment.map_or(&[][..], ParticipationAssessment::evidence_refs),
        input_generation: participation.input_generation(),
    };
    budget.record(&row)?;
    Ok(row)
}

fn disposition(outcome: ParticipationExclusionOutcome) -> &'static str {
    match outcome {
        ParticipationExclusionOutcome::Excluded => "excluded",
        ParticipationExclusionOutcome::Unresolved => "unresolved",
    }
}

fn assessment<'a>(
    value: &ParticipationExclusion,
    participation: &'a VerifiedParticipationView,
) -> ReportResult<Option<&'a ParticipationAssessment>> {
    if matches!(
        value.reason(),
        ParticipationExclusionReason::DocumentarySchoolBindingUnresolved { .. }
    ) {
        return documentary::assessment(value, participation).map(Some);
    }
    match (
        value.reason(),
        participation.assessment(value.performance_id()),
    ) {
        (ParticipationExclusionReason::MissingParticipationEvidence, None)
            if value.outcome() == ParticipationExclusionOutcome::Unresolved =>
        {
            Ok(None)
        }
        (ParticipationExclusionReason::Assessment(reasons), Some(assessment))
            if reasons == assessment.reasons() =>
        {
            ensure_assessment(assessment)?;
            let outcome = match assessment.outcome() {
                ParticipationOutcome::Eligible => {
                    return Err(defect(
                        "eligible participation assessment cannot be excluded",
                    ));
                }
                ParticipationOutcome::Excluded => ParticipationExclusionOutcome::Excluded,
                ParticipationOutcome::Unresolved => ParticipationExclusionOutcome::Unresolved,
            };
            if outcome != value.outcome() {
                return Err(defect(
                    "participation exclusion disposition contradicts its assessment",
                ));
            }
            Ok(Some(assessment))
        }
        _ => Err(defect(
            "participation exclusion reasons contradict its assessment",
        )),
    }
}

struct Reasons(ParticipationExclusionReason);

impl Serialize for Reasons {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            ParticipationExclusionReason::MissingParticipationEvidence => {
                serializer.collect_seq(["missing_participation_evidence"])
            }
            ParticipationExclusionReason::Assessment(reasons)
            | ParticipationExclusionReason::DocumentarySchoolBindingUnresolved {
                reasons, ..
            } => serializer.collect_seq(reasons.iter()),
        }
    }
}

fn encode(value: &impl Serialize) -> ReportResult<String> {
    serde_json::to_string(value)
        .map_err(|error| defect(format!("encoding participation exclusion: {error}")))
}

fn defect(detail: impl Into<String>) -> ReportError {
    ReportError::Invariant {
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests;
