use crate::bests;
use crate::export::ExportDataset;
use crate::report::{EventAdmission, ReportError, Scope};
use crate::workbook::{self, Censuses, Options};
use census_domain::model::{
    CatalogError, EventExclusionReason, EventKind, EventResolutionReason, Mark, SchoolYear,
};
use serde_json::Value;
use std::path::{Path, PathBuf};

mod fixture;
mod mutate;
mod readback;
mod scope;
mod transition;
use fixture::{fixture, Fixture, TestResult};

fn options() -> TestResult<Options> {
    Ok(Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("fixture school year")?,
        jurisdictions: Vec::new(),
    })
}

fn best_options() -> bests::Options {
    bests::Options {
        scope: Scope::AllSources,
        grad_year: Some(2027),
        limit: None,
        jurisdictions: Vec::new(),
    }
}

fn publish(fixture: &Fixture, dataset: &ExportDataset) -> TestResult<PathBuf> {
    let options = options()?;
    let censuses = Censuses::of(dataset, &fixture.store.out_dir(), &[]);
    Ok(workbook::build_from(
        dataset,
        &fixture.store,
        &options,
        &censuses,
    )?)
}

#[test]
fn whole_meet_publishes_one_individual_one_team_relay_and_all_retained_dispositions() -> TestResult
{
    let fixture = fixture()?;
    let dataset = ExportDataset::load(&fixture.store)?;
    let raw_events = census_domain::model::serialized_bytes(&dataset.events)?;
    let raw_performances = census_domain::model::serialized_bytes(&dataset.performances)?;
    let admission = EventAdmission::of(&dataset)?;
    check!(eq; admission.counts().included, 2);
    check!(eq; admission.counts().excluded.get(&EventExclusionReason::WheelchairRace), Some(&2));
    check!(eq; admission.counts().excluded.get(&EventExclusionReason::CompetitiveWalking), Some(&1));
    check!(eq; admission.counts().unresolved.get(&EventResolutionReason::UnsupportedScholasticVariant), Some(&1));
    let bests = bests::build_from_dataset(&dataset, &best_options())?;
    check!(eq; bests.len(), 1);
    check!(eq; bests.first().ok_or("ordinary best")?.key.event_kind, EventKind::Track100m);
    check!(eq; bests.first().ok_or("ordinary athlete")?.athlete_id(), &fixture.athlete);
    let winner = bests.first().ok_or("ordinary winner")?;
    check!(eq; winner.result.value, 10_940_000_000);
    check!(eq; winner.mark_text(), "10.94");
    check!(eq; winner.source.source_key, fixture.ordinary.as_str());
    check!(eq; winner.source.result_url, "https://example.test/wiaa_results/catalog");
    check!(eq; winner.population.marks, 1);
    check!(eq; winner.population.sources, 1);
    let path = publish(&fixture, &dataset)?;
    let generation = path.parent().ok_or("published generation")?;
    super::verify_sidecars::verify(generation, &dataset, &options()?)?;
    workbook::verify::verify_frozen(&path, &dataset, &options()?)?;
    mutate::assert_cell(&path, "Athletes", "Performance count", "1")?;
    let relays = lines(&generation.join("relays.jsonl"))?;
    check!(eq; relays.len(), 1);
    let relay = relays.first().ok_or("published team relay")?;
    check!(eq; relay.pointer("/result/id").and_then(Value::as_str), Some(fixture.relay.as_str()));
    check!(eq; relay.pointer("/result/members/0/order").and_then(Value::as_u64), Some(3));
    check!(eq; relay.pointer("/result/members/1/name_as_published").and_then(Value::as_str), Some("Second published runner"));
    let published = dataset
        .relays
        .iter()
        .find(|result| result.id == fixture.relay)
        .ok_or("retained published membership")?;
    check!(eq; relay.pointer("/result/members"), Some(&serde_json::to_value(&published.members)?));
    check!(eq; census_domain::model::serialized_bytes(&dataset.events)?, raw_events);
    check!(eq; census_domain::model::serialized_bytes(&dataset.performances)?, raw_performances);
    drop(admission);
    drop(dataset);
    let reopened = ExportDataset::reopen_frozen(&generation.join("frozen-input.json"))?;
    check!(eq; reopened.relays.len(), 2);
    check!(eq; reopened.events.len(), 6);
    check!(eq; reopened.performances.len(), 4);
    check!(eq; census_domain::model::serialized_bytes(&reopened.events)?, raw_events);
    super::verify_sidecars::verify(generation, &reopened, &options()?)?;
    Ok(())
}

#[derive(Clone, Copy)]
enum HiddenBy {
    RawMark,
    MissingAthlete,
    RelayMark,
    NoPerformance,
    WrongCohort,
}

#[test]
fn dataset_admission_refuses_oversize_labels_before_every_candidate_skip() -> TestResult {
    for hidden in [
        HiddenBy::RawMark,
        HiddenBy::MissingAthlete,
        HiddenBy::RelayMark,
        HiddenBy::NoPerformance,
        HiddenBy::WrongCohort,
    ] {
        let fixture = fixture()?;
        let mut dataset = ExportDataset::load(&fixture.store)?;
        dataset
            .events
            .iter_mut()
            .find(|event| event.id == fixture.ordinary)
            .ok_or("ordinary event")?
            .source_labels
            .first_mut()
            .ok_or("owned raw label")?
            .label = "x".repeat(4097);
        match hidden {
            HiddenBy::RawMark => {
                for performance in &mut dataset.performances {
                    performance.mark = Mark::Raw("DNF".into());
                }
            }
            HiddenBy::MissingAthlete => dataset.athletes.clear(),
            HiddenBy::RelayMark => {
                for event in &mut dataset.events {
                    event.kind = EventKind::Relay4x100;
                }
            }
            HiddenBy::NoPerformance => dataset.performances.clear(),
            HiddenBy::WrongCohort => {
                for athlete in &mut dataset.athletes {
                    athlete.published_graduations.clear();
                }
            }
        }
        let error = bests::build_from_dataset(&dataset, &best_options())
            .err()
            .ok_or("hidden catalog error")?;
        check!(matches!(
            error,
            ReportError::Catalog {
                source: CatalogError::LabelBytesExceeded,
                ..
            }
        ));
    }
    Ok(())
}

#[test]
fn independent_oracle_rejects_missing_catalog_family_and_extra_excluded_best_or_relay() -> TestResult
{
    for mutation in [
        "missing_family",
        "extra_best",
        "extra_relay",
        "reordered_relay",
    ] {
        let fixture = fixture()?;
        let dataset = ExportDataset::load(&fixture.store)?;
        let path = publish(&fixture, &dataset)?;
        let generation = path.parent().ok_or("published generation")?;
        match mutation {
            "missing_family" => {
                let path = generation.join("event-catalog.json");
                let mut audit: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
                audit
                    .get_mut("events")
                    .and_then(Value::as_array_mut)
                    .ok_or("event audit")?
                    .retain(|event| {
                        event.get("event").and_then(Value::as_str)
                            != Some(fixture.unresolved.as_str())
                    });
                std::fs::write(path, serde_json::to_vec(&audit)?)?;
            }
            "extra_best" => {
                let path = generation.join("best-results-co2027.jsonl");
                let mut rows = lines(&path)?;
                let mut forged = rows.first().ok_or("positive best row")?.clone();
                forged["key"]["event_kind"] = serde_json::to_value(EventKind::Unmapped {
                    label: "Wheelchair 100m".into(),
                })?;
                rows.push(forged);
                mutate::write_lines(&path, &rows)?;
            }
            "extra_relay" => {
                let path = generation.join("relays.jsonl");
                let mut rows = lines(&path)?;
                let mut forged = rows.first().ok_or("positive relay")?.clone();
                forged["result"]["event"] = serde_json::to_value(&fixture.excluded)?;
                rows.push(forged);
                mutate::write_lines(&path, &rows)?;
            }
            "reordered_relay" => {
                let path = generation.join("relays.jsonl");
                let mut rows = lines(&path)?;
                rows.first_mut()
                    .ok_or("positive relay")?
                    .pointer_mut("/result/members")
                    .and_then(Value::as_array_mut)
                    .ok_or("published members")?
                    .sort_by_key(|member| member.get("order").and_then(Value::as_u64));
                mutate::write_lines(&path, &rows)?;
            }
            _ => return Err("unknown finite mutation".into()),
        }
        check!(super::verify_sidecars::verify(generation, &dataset, &options()?).is_err());
    }
    Ok(())
}

#[test]
fn independent_workbook_oracle_rejects_missing_review_family_and_extra_relay_row() -> TestResult {
    for sheet in ["Review", "Relays"] {
        let fixture = fixture()?;
        let dataset = ExportDataset::load(&fixture.store)?;
        let original = publish(&fixture, &dataset)?;
        let forged = fixture
            .directory
            .path()
            .join(format!("forged-{sheet}.xlsx"));
        mutate::workbook(&original, &forged, sheet)?;
        check!(workbook::verify::verify_frozen(&forged, &dataset, &options()?).is_err());
    }
    Ok(())
}

fn lines(path: &Path) -> TestResult<Vec<Value>> {
    std::fs::read_to_string(path)?
        .lines()
        .map(|line| Ok(serde_json::from_str(line)?))
        .collect()
}

#[test]
fn semantic_readback_rejects_wrong_marks_missing_rows_and_same_count_excluded_replacements(
) -> TestResult {
    for mutation in [
        "wrong_best_mark",
        "missing_best",
        "excluded_best",
        "wrong_relay_mark",
        "missing_relay",
        "excluded_relay",
    ] {
        let fixture = fixture()?;
        let dataset = ExportDataset::load(&fixture.store)?;
        let path = publish(&fixture, &dataset)?;
        let generation = path.parent().ok_or("published generation")?;
        let file = if mutation.ends_with("relay") || mutation == "wrong_relay_mark" {
            "relays.jsonl"
        } else {
            "best-results-co2027.jsonl"
        };
        let path = generation.join(file);
        let mut rows = lines(&path)?;
        check!(eq; rows.len(), 1);
        match mutation {
            "wrong_best_mark" => rows.first_mut().ok_or("best")?["value"] = Value::from(1),
            "missing_best" | "missing_relay" => rows.clear(),
            "excluded_best" => {
                rows.first_mut().ok_or("best")?["key"]["event_kind"] =
                    serde_json::to_value(EventKind::Unmapped {
                        label: "Wheelchair 100m".into(),
                    })?
            }
            "wrong_relay_mark" => {
                rows.first_mut().ok_or("relay")?["result"]["mark"] = serde_json::to_value(
                    Mark::TimeSeconds(census_domain::model::ExactSeconds::parse("1.00")?),
                )?
            }
            "excluded_relay" => {
                rows.first_mut().ok_or("relay")?["result"]["event"] =
                    serde_json::to_value(&fixture.excluded)?
            }
            _ => return Err("unknown finite semantic mutation".into()),
        }
        mutate::write_lines(&path, &rows)?;
        let error = super::verify_sidecars::verify(generation, &dataset, &options()?)
            .err()
            .ok_or("semantic corruption accepted")?;
        check!(error.to_string().contains(file));
    }
    Ok(())
}
