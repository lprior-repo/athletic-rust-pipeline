use super::fixture::{fixture, Fixture, TestResult};
use crate::export::ExportDataset;
use crate::report::{
    Derivation, EventAdmission, RelayDisposition, RelayGap, RelayProjection, Scope,
};
use census_domain::model::{Evidence, SourceRef};
use std::path::{Path, PathBuf};

fn core_dataset(fixture: &Fixture) -> TestResult<ExportDataset> {
    let mut dataset = ExportDataset::load(&fixture.store)?;
    let source = SourceRef::new(
        "wiaa_results",
        Some("https://example.test/catalog/core".into()),
    );
    for athlete in &mut dataset.athletes {
        athlete
            .evidence
            .push(Evidence::parsed(source.clone(), "2025-03-01"));
    }
    for meet in &mut dataset.meets {
        meet.evidence
            .push(Evidence::parsed(source.clone(), "2025-03-01"));
    }
    Ok(dataset)
}

#[derive(Clone, Copy)]
enum NonCore {
    Result,
    Event,
    Meet,
}

#[test]
fn core_relays_refuse_non_core_result_event_or_meet_without_losing_raw_rows() -> TestResult {
    for changed in [NonCore::Result, NonCore::Event, NonCore::Meet] {
        let fixture = fixture()?;
        let mut dataset = core_dataset(&fixture)?;
        let relay = dataset
            .relays
            .iter()
            .find(|relay| relay.id == fixture.relay)
            .ok_or("relay")?;
        let event_id = relay.event.clone();
        let meet_id = relay.meet.clone();
        match changed {
            NonCore::Result => {
                let relay = dataset
                    .relays
                    .iter_mut()
                    .find(|relay| relay.id == fixture.relay)
                    .ok_or("relay")?;
                for evidence in &mut relay.evidence {
                    evidence.source.id = "athleticnet".into();
                }
            }
            NonCore::Event => {
                let event = dataset
                    .events
                    .iter_mut()
                    .find(|event| event.id == event_id)
                    .ok_or("event")?;
                for label in &mut event.source_labels {
                    label.source.id = "athleticnet".into();
                }
                for evidence in &mut event.evidence {
                    evidence.source.id = "athleticnet".into();
                }
            }
            NonCore::Meet => {
                let meet = dataset
                    .meets
                    .iter_mut()
                    .find(|meet| meet.id == meet_id)
                    .ok_or("meet")?;
                for evidence in &mut meet.evidence {
                    evidence.source.id = "athleticnet".into();
                }
            }
        }
        let raw = census_domain::model::serialized_bytes(&dataset.relays)?;
        let admission = EventAdmission::of(&dataset)?;
        check!(admission.included_kind(&event_id).is_some());
        let all = Derivation::of(&dataset, Scope::AllSources, Some(2027), &[]);
        let all_projection = RelayProjection::of(&all, &admission)?;
        check!(eq; all_projection.selections.len(), 1);
        check!(eq; all_projection.selections.first().ok_or("all-source relay")?.result.id, fixture.relay);

        let core = Derivation::of(&dataset, Scope::Core, Some(2027), &[]);
        check!(eq; core.athletes().len(), 1);
        let core_projection = RelayProjection::of(&core, &admission)?;
        check!(core_projection.selections.is_empty());
        check!(eq; core_projection.admissions.len(), dataset.relays.len());
        let refused = core_projection
            .admissions
            .iter()
            .find(|row| row.relay == &fixture.relay)
            .ok_or("retained refusal")?;
        check!(matches!(
            refused.disposition,
            RelayDisposition::Unresolved(RelayGap::OutsideEvidenceScope)
        ));
        check!(eq; census_domain::model::serialized_bytes(&dataset.relays)?, raw);
    }
    Ok(())
}

#[test]
fn core_summary_and_readback_agree_when_an_included_event_has_only_non_core_evidence() -> TestResult
{
    let fixture = fixture()?;
    let mut dataset = core_dataset(&fixture)?;
    let event = dataset
        .events
        .iter_mut()
        .find(|event| event.id == fixture.ordinary)
        .ok_or("ordinary event")?;
    for label in &mut event.source_labels {
        label.source.id = "athleticnet".into();
    }
    for evidence in &mut event.evidence {
        evidence.source.id = "athleticnet".into();
    }
    let admission = EventAdmission::of(&dataset)?;
    check!(admission.included_kind(&fixture.ordinary).is_some());
    let mut options = super::options()?;
    options.scope = Scope::Core;
    let core = render(&dataset, fixture.directory.path(), &options)?;
    super::mutate::assert_cell(&core, "Athletes", "Performance count", "0")?;
    super::mutate::assert_cell(&core, "Athletes", "Meet count", "0")?;
    crate::workbook::verify::verify_frozen(&core, &dataset, &options)?;
    options.scope = Scope::AllSources;
    let all = render(&dataset, fixture.directory.path(), &options)?;
    super::mutate::assert_cell(&all, "Athletes", "Performance count", "1")?;
    crate::workbook::verify::verify_frozen(&all, &dataset, &options)?;
    Ok(())
}

fn render(
    dataset: &ExportDataset,
    directory: &Path,
    options: &crate::workbook::Options,
) -> TestResult<PathBuf> {
    let path = directory.join(format!("scope-{}.xlsx", options.scope.as_str()));
    let admission = EventAdmission::of(dataset)?;
    let selected = crate::workbook::select_bests(&admission, options)?;
    let derivation = Derivation::of(
        dataset,
        options.scope,
        options.grad_year,
        &options.jurisdictions,
    );
    let population = Derivation::of(dataset, options.scope, None, &options.jurisdictions);
    let recruiting = crate::workbook::recruiting::Recruiting::of(
        &derivation,
        options.school_year,
        selected,
        &admission,
    )?;
    let censuses = crate::workbook::Censuses::of(dataset, directory, &options.jurisdictions);
    let views = crate::workbook::Views::of(
        &censuses,
        &recruiting,
        &derivation,
        &population,
        options.school_year,
        &admission,
    );
    crate::workbook::write_workbook(&path, views)?;
    Ok(path)
}
