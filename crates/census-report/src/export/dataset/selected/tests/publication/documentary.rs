use super::TestResult;
use crate::bests::{ParticipationExclusionOutcome, ParticipationExclusionReason};
use crate::export::{ExportDataset, ParticipationDiagnostic, ParticipationOutcome};
use crate::report::Scope;
use crate::workbook::{self, publication, Censuses, Options};
use census_domain::model::GradYear;
use census_domain::participation_candidates::{CandidateOutcome, ParticipationReason};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

mod fixture;
mod lifecycle;
mod refusals;
mod roundtrip;

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
    check!(eq; dataset.selected_authority()?.profile().input().graduation_years, vec![GradYear::CO2027]);
    check!(eq; expected.len(), 3);
    check!(dataset.performances.is_empty());
    check!(dataset.teams.is_empty());
    let view = dataset.verified_participation()?;
    check!(view.unsupported_documentary().is_empty());
    check!(eq; view.documentary_rows().count(), 3);
    let options = bests_options(dataset)?;
    check!(eq; options.grad_year, Some(2027));
    check!(Some(athlete.grad_year.get()) != options.grad_year);
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
    let reasons = vec![
        ParticipationReason::CohortUnresolved,
        ParticipationReason::ParticipationContextUnresolved,
        ParticipationReason::HighSchoolStatusMissing,
        ParticipationReason::SummerBoundaryUnresolved,
    ];
    let mut observed = Vec::new();
    for (source, exclusion) in expected.iter().zip(&outcome.participation_exclusions) {
        check!(eq; source.athlete, athlete.id);
        let row = view
            .documentary(&source.performance)
            .ok_or("genuine documentary observation")?;
        check!(eq; row.event_id(), &source.event);
        check!(eq; row.athlete_id(), &source.athlete);
        check!(eq; row.domain_outcome(), CandidateOutcome::Unresolved);
        check!(eq; row.assessment().outcome(), ParticipationOutcome::Unresolved);
        check!(eq; row.assessment().reasons().iter().collect::<Vec<_>>(), reasons);
        check!(eq; row.assessment().evidence_refs(), source.references);
        check!(eq; row.assessment().diagnostics(), &[ParticipationDiagnostic::DocumentaryAssessmentUnresolved, ParticipationDiagnostic::SchoolBindingUnresolved]);
        check!(view.assessment(&source.performance).is_none());
        check!(eq; exclusion.performance_id(), &source.performance);
        check!(eq; exclusion.event_id(), &source.event);
        check!(eq; exclusion.athlete_id(), &source.athlete);
        check!(eq; exclusion.outcome(), ParticipationExclusionOutcome::Unresolved);
        check!(eq; exclusion.reason(), ParticipationExclusionReason::DocumentarySchoolBindingUnresolved { assessment: CandidateOutcome::Unresolved, reasons: row.assessment().reasons() });
        observed.push(json!({"performance":source.performance,"event":source.event,"athlete":source.athlete,"actual":"Unresolved","disposition":"unresolved","reasons":reasons,"diagnostics":row.assessment().diagnostics(),"references":row.assessment().evidence_refs()}));
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
        check!(eq; row.get(3), Some("unresolved"));
        check!(eq; row.get(4).ok_or("reasons")?, serde_json::to_string(&assessed.reasons().iter().collect::<Vec<_>>())?);
        check!(eq; row.get(5), Some("[\"documentary_assessment_unresolved\",\"school_binding_unresolved\"]"));
        check!(eq; row.get(6).ok_or("references")?, serde_json::to_string(&source.references)?);
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
        crate::export::qualification::participation::ParticipationReplay::V3(_)
    ));
    let wire = serde_json::to_value(replay)?;
    Ok(wire
        .pointer("/attachment/object")
        .and_then(Value::as_str)
        .ok_or("required V3 closure")?
        .into())
}
