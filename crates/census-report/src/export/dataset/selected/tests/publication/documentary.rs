use super::TestResult;
use crate::bests::{ParticipationExclusionOutcome, ParticipationExclusionReason};
use crate::export::{ExportDataset, ParticipationDiagnostic, ParticipationOutcome};
use crate::report::Scope;
use crate::workbook::{self, publication, Censuses, Options};
use census_domain::model::GradYear;
use census_domain::participation_candidates::{CandidateOutcome, ParticipationReason};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

mod acquisition_cutoff;
mod calendar;
mod fixture;
mod historical;
mod historical_retained;
mod identity_lifecycle;
mod lifecycle;
mod refusals;
mod roundtrip;
mod school;

use fixture::{Expected, Fixture};

fn inspect(dataset: &ExportDataset, expected: &[Expected]) -> TestResult<Value> {
    let [athlete] = dataset.athletes.as_slice() else {
        return Err("one genuine retained athlete".into());
    };
    check!(eq; athlete.grad_year.get(), 2026);
    check!(eq; athlete.identity_in(&census_domain::model::SourceNamespace::MilesplitAthlete)
        .ok_or("genuine retained provider identity")?.id, "7223777");
    check!(dataset
        .verified_public_identities()?
        .public_id(athlete.id.as_str())
        .is_some());
    let selected = dataset
        .selected_authority()?
        .profile()
        .input()
        .graduation_years
        .clone();
    check!(
        selected == vec![GradYear::CO2027]
            || selected == vec![GradYear::new(2026).ok_or("controlled CO2026 year")?]
    );
    check!(eq; expected.len(), 3);
    check!(dataset.performances.is_empty());
    check!(dataset.teams.is_empty());
    let view = dataset.verified_participation()?;
    check!(view.unsupported_documentary().is_empty());
    check!(eq; view.documentary_rows().count(), 3);
    let options = bests_options(dataset)?;
    let (actual, mapped, disposition, tag) = match options.grad_year {
        Some(2027) => (
            CandidateOutcome::Excluded,
            ParticipationOutcome::Excluded,
            ParticipationExclusionOutcome::Excluded,
            ParticipationDiagnostic::DocumentaryAssessmentExcluded,
        ),
        Some(2026) => (
            CandidateOutcome::Unresolved,
            ParticipationOutcome::Unresolved,
            ParticipationExclusionOutcome::Unresolved,
            ParticipationDiagnostic::DocumentaryAssessmentUnresolved,
        ),
        _ => return Err("controlled selected cohort".into()),
    };
    let derivation = crate::report::Derivation::of(
        dataset,
        options.scope,
        options.grad_year,
        &options.jurisdictions,
    )?;
    check!(derivation.athletes().is_empty());
    check!(derivation.performances().is_empty());
    let outcome = crate::bests::build_with_exclusions(dataset, &options)?;
    check!(outcome.selections.is_empty());
    check!(eq; outcome.participation_exclusions.len(), 3);
    let mut reasons = Vec::new();
    if options.grad_year == Some(2027) {
        reasons.push(ParticipationReason::CohortOutsideSelection);
    }
    reasons.extend([
        ParticipationReason::ParticipationContextUnresolved,
        ParticipationReason::SummerBoundaryUnresolved,
    ]);
    let mut observed = Vec::new();
    for (source, exclusion) in expected.iter().zip(&outcome.participation_exclusions) {
        check!(eq; source.athlete, athlete.id);
        let row = view
            .documentary(&source.performance)
            .ok_or("genuine documentary observation")?;
        check!(eq; row.event_id(), &source.event);
        check!(eq; row.athlete_id(), &source.athlete);
        check!(eq; row.domain_outcome(), actual);
        check!(eq; row.assessment().outcome(), mapped);
        check!(eq; row.assessment().reasons().iter().collect::<Vec<_>>(), reasons);
        check!(eq; row.assessment().evidence_refs(), expected_references(source));
        check!(eq; row.assessment().diagnostics(), &[tag, ParticipationDiagnostic::SchoolBindingUnresolved]);
        check!(view.assessment(&source.performance).is_none());
        check!(eq; exclusion.performance_id(), &source.performance);
        check!(eq; exclusion.event_id(), &source.event);
        check!(eq; exclusion.athlete_id(), &source.athlete);
        check!(eq; exclusion.outcome(), disposition);
        check!(eq; exclusion.reason(), ParticipationExclusionReason::DocumentarySchoolBindingUnresolved { assessment: actual, reasons: row.assessment().reasons() });
        observed.push(json!({"performance":source.performance,"event":source.event,"athlete":source.athlete,"actual":format!("{actual:?}"),"disposition":format!("{disposition:?}"),"reasons":reasons,"diagnostics":row.assessment().diagnostics(),"references":row.assessment().evidence_refs()}));
    }
    Ok(
        json!({"generation":view.input_generation(),"profile":view.profile_id(),"rule":view.participation_rule(),"rows":observed}),
    )
}

fn bests_options(dataset: &ExportDataset) -> TestResult<crate::bests::Options> {
    let profile = dataset.selected_authority()?.profile();
    let [cohort] = profile.input().graduation_years.as_slice() else {
        return Err("one genuine selected cohort".into());
    };
    Ok(crate::bests::Options {
        scope: Scope::AllSources,
        grad_year: Some(cohort.get()),
        jurisdictions: profile.input().jurisdictions.clone(),
        limit: None,
    })
}

fn publish(fixture: &Fixture, dataset: &ExportDataset, root: &Path) -> TestResult<PathBuf> {
    let bests = bests_options(dataset)?;
    let options = Options {
        address_corpus: None,
        grad_year: bests.grad_year,
        out: Some(root.to_path_buf()),
        limit: None,
        scope: bests.scope,
        jurisdictions: bests.jurisdictions,
        school_year: fixture.profile.input().contact_school_year,
    };
    let censuses = Censuses::of(dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    Ok(workbook::build_from(
        dataset,
        &fixture.store,
        &options,
        &censuses,
    )?)
}

fn csv(directory: &Path, dataset: &ExportDataset, expected: &[Expected]) -> TestResult {
    let mut reader = csv::Reader::from_path(directory.join("participation_exclusions.csv"))?;
    check!(eq; reader.headers()?.iter().collect::<Vec<_>>(), vec!["retained_candidate_id","performance_id","event_id","disposition","reasons","diagnostics","evidence_refs","input_generation"]);
    let rows = reader.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; rows.len(), 3);
    let view = dataset.verified_participation()?;
    for (row, source) in rows.iter().zip(expected) {
        let assessed = view
            .documentary(&source.performance)
            .ok_or("genuine source assessment")?
            .assessment();
        check!(eq; row.len(), 8);
        check!(eq; row.get(0), Some(source.athlete.as_str()));
        check!(eq; row.get(1), Some(source.performance.as_str()));
        check!(eq; row.get(2), Some(source.event.as_str()));
        check!(eq; row.get(3), Some("excluded"));
        check!(eq; row.get(4).ok_or("reasons")?, serde_json::to_string(&assessed.reasons().iter().collect::<Vec<_>>())?);
        check!(eq; row.get(5), Some("[\"documentary_assessment_excluded\",\"school_binding_unresolved\"]"));
        check!(eq; row.get(6).ok_or("references")?, serde_json::to_string(&expected_references(source))?);
        check!(eq; row.get(7), Some(view.input_generation()));
    }
    Ok(())
}

fn attachment_object(dataset: &ExportDataset, directory: &Path) -> TestResult<String> {
    let proof = serde_json::to_value(dataset.selected_proofs.as_ref().ok_or("selected proofs")?)?;
    let descriptor = proof
        .get("participation_digest")
        .and_then(Value::as_str)
        .ok_or("participation descriptor")?;
    let replay = crate::export::qualification::participation::ParticipationReplay::load(
        directory, descriptor,
    )?;
    check!(matches!(
        &replay,
        crate::export::qualification::participation::ParticipationReplay::V4(_)
    ));
    let wire = serde_json::to_value(replay)?;
    Ok(wire
        .pointer("/attachment/object")
        .and_then(Value::as_str)
        .ok_or("required V4 closure")?
        .into())
}

fn expected_references(source: &Expected) -> Vec<String> {
    source
        .references
        .iter()
        .chain(&source.semantic_references)
        .cloned()
        .collect()
}
