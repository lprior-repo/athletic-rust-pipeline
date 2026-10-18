use super::super::source::TestResult;
use super::artifacts;
use crate::bests::{ParticipationExclusionOutcome, ParticipationExclusionReason, SelectionOutcome};
use crate::export::ExportDataset;
use census_domain::model::CanonicalPerformance;
use std::path::Path;
pub(super) fn rows(
    directory: &Path,
    dataset: &ExportDataset,
    outcome: &SelectionOutcome,
    wanted: &CanonicalPerformance,
) -> TestResult<()> {
    let rows = artifacts::csv_rows(&directory.join("participation_exclusions.csv"))?;
    fixture_membership(&rows, dataset, wanted)?;
    check!(eq; rows.len(), outcome.participation_exclusions.len());
    let view = dataset.verified_participation()?;
    for value in &outcome.participation_exclusions {
        let row = artifacts::unique(&rows, "performance_id", value.performance_id().as_str())?;
        let disposition = match value.outcome() {
            ParticipationExclusionOutcome::Excluded => "excluded",
            ParticipationExclusionOutcome::Unresolved => "unresolved",
        };
        artifacts::fields(
            row,
            &[
                ("retained_candidate_id", value.athlete_id().as_str()),
                ("performance_id", value.performance_id().as_str()),
                ("event_id", value.event_id().as_str()),
                ("disposition", disposition),
                ("input_generation", view.input_generation()),
            ],
        )?;
        match value.reason() {
            ParticipationExclusionReason::MissingParticipationEvidence => {
                check!(value.performance_id() != &wanted.id);
                artifacts::fields(
                    row,
                    &[
                        ("reasons", "[\"missing_participation_evidence\"]"),
                        ("diagnostics", "[]"),
                        ("evidence_refs", "[]"),
                    ],
                )?;
                check!(view.assessment(value.performance_id()).is_none());
            }
            ParticipationExclusionReason::Assessment(reasons) => {
                let encoded = serde_json::to_string(&reasons.iter().collect::<Vec<_>>())?;
                artifacts::fields(row, &[("reasons", encoded.as_str())])?;
                let assessment = view
                    .assessment(value.performance_id())
                    .ok_or("excluded assessment")?;
                check!(eq; assessment.reasons(), reasons);
                let diagnostics = serde_json::to_string(assessment.diagnostics())?;
                let refs = serde_json::to_string(assessment.evidence_refs())?;
                artifacts::fields(
                    row,
                    &[
                        ("diagnostics", diagnostics.as_str()),
                        ("evidence_refs", refs.as_str()),
                    ],
                )?;
            }
            ParticipationExclusionReason::DocumentarySchoolBindingUnresolved { .. } => {
                return Err("controlled exclusions use direct window and missing gap".into());
            }
        }
    }
    let headers = artifacts::headers_of(&directory.join("participation_exclusions.csv"))?;
    for name in [
        "retained_candidate_id",
        "performance_id",
        "event_id",
        "disposition",
        "reasons",
        "diagnostics",
        "evidence_refs",
        "input_generation",
    ] {
        check!(headers.iter().any(|value| value == name));
    }
    Ok(())
}

fn fixture_membership(
    rows: &[artifacts::Row],
    dataset: &ExportDataset,
    wanted: &CanonicalPerformance,
) -> TestResult<()> {
    let feb = artifacts::by_mark(dataset, "49.00", "2026-02-28")?;
    let may50 = artifacts::by_mark(dataset, "50.00", "2026-05-15")?;
    let may51 = artifacts::by_mark(dataset, "51.00", "2026-05-16")?;
    let feb_row = artifacts::unique(rows, "performance_id", feb.id.as_str())?;
    artifacts::fields(
        feb_row,
        &[
            ("disposition", "excluded"),
            ("reasons", "[\"outside_participation_window\"]"),
            ("diagnostics", "[]"),
        ],
    )?;
    if wanted.id == may50.id {
        check!(eq; rows.len(), 1);
    } else {
        check!(eq; wanted.id, may51.id);
        check!(eq; rows.len(), 2);
        let revoked = artifacts::unique(rows, "performance_id", may50.id.as_str())?;
        artifacts::fields(
            revoked,
            &[
                ("disposition", "unresolved"),
                (
                    "reasons",
                    "[\"participation_context_unresolved\",\"high_school_status_missing\"]",
                ),
                ("diagnostics", "[]"),
            ],
        )?;
    }
    Ok(())
}
