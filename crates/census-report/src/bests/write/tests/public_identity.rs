use super::*;
use crate::export::public_identity::tests::fixture::Fixture;
use crate::export::selected_test_fixture::consumer::{capture, merge, source_births};
use census_domain::model::{CanonicalAthlete, EventSemantics};
use census_store::{Store, StoreError};

pub(crate) fn row(athlete: &CanonicalAthlete) -> SharedSelection {
    SharedSelection {
        key: PrKey {
            athlete_id: athlete.id.clone(),
            ..key()
        },
        source: SelectionSource {
            source_athlete: athlete
                .source
                .as_ref()
                .map_or_else(String::new, |identity| identity.id.clone()),
            source_key: "synthetic-public-consumer-original-source-key".into(),
            ..source(&athlete.canonical_name)
        },
        ..selection(
            &athlete.canonical_name,
            "Synthetic School",
            "Synthetic Meet",
            Mark::Raw("11.00".into()),
            None,
        )
    }
}

#[test]
fn selected_bests_publish_public_join_keys_and_keep_original_selection_and_source_identity(
) -> TestResult {
    let fixture = source_births()?;
    let (view, _proofs) = capture(&fixture.store)?;
    let [first, _, _] = &fixture.athletes;
    let [birth, _] = &fixture.births;
    let rows = [row(first)];
    let original = serde_json::to_value(&rows)?;
    let (jsonl, csv) = super::super::selected::write(
        &fixture.directory.path().join("publication"),
        &rows,
        "co2027",
        EventSemantics::ResultGroups21,
        &view,
    )?;
    let wire: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(jsonl)?)?;
    check!(eq; wire.get("schema"), Some(&serde_json::json!("selected_identity_v2")));
    check!(eq; wire.get("athlete_id"), Some(&serde_json::json!(birth.public_id())));
    check!(eq; wire.get("canonical_candidate_id"), Some(&serde_json::json!(first.id)));
    check!(eq; wire.get("selection"), Some(&serde_json::to_value(rows.first().ok_or("selection")?)?));
    check!(eq; wire.get("identity_resolution"), Some(&serde_json::json!({
        "state": "canonical", "successor": {
            "public_id": birth.public_id(), "representative_candidate": first.id,
            "established": { "sequence": 1, "position": 0 },
        },
    })));
    let mut csv = csv::Reader::from_path(csv)?;
    let header = csv.headers()?.clone();
    let record = csv.records().next().ok_or("CSV row")??;
    check!(eq; header.len(), 44);
    check!(eq; record.get(column(&header, "athlete_id")?), Some(birth.public_id().as_str()));
    check!(eq; record.get(column(&header, "canonical_candidate_id")?), Some(first.id.as_str()));
    check!(eq; record.get(column(&header, "source_athlete")?), first.source.as_ref().map(|identity| identity.id.as_str()));
    check!(eq; record.get(column(&header, "source_key")?), Some("synthetic-public-consumer-original-source-key"));
    check!(eq; serde_json::to_value(&rows)?, original);
    Ok(())
}

#[test]
fn selected_bests_preserve_premerge_identity_snapshot_and_replay_real_merge_after_reopen(
) -> TestResult {
    let fixture = source_births()?;
    let (old_view, _old_proofs) = capture(&fixture.store)?;
    let [_, second, _] = &fixture.athletes;
    let rows = [row(second)];
    let old_head = old_view.verified_public_identities()?.head().clone();
    let old = serde_json::to_value(
        crate::export::public_identity::output::AcceptedIdentityFields::for_candidate(
            old_view.verified_public_identities()?,
            second.id.as_str(),
        )?,
    )?;
    let [first_birth, second_birth] = &fixture.births;
    check!(eq; old.get("athlete_id"), Some(&serde_json::json!(second_birth.public_id())));
    check!(eq; old_view.verified_current_affiliations()?.resolution(&second.id), &crate::export::CurrentAffiliationResolution::Unknown);
    merge(&fixture)?;
    let (merged_view, _merged_proofs) = capture(&fixture.store)?;
    let held = serde_json::to_value(
        crate::export::public_identity::output::AcceptedIdentityFields::for_candidate(
            old_view.verified_public_identities()?,
            second.id.as_str(),
        )?,
    )?;
    check!(eq; held, old);
    check!(eq; old_view.verified_public_identities()?.head(), &old_head);
    let (merged_json, merged_csv) = super::super::selected::write(
        &fixture.directory.path().join("merged"),
        &rows,
        "co2027",
        EventSemantics::ResultGroups21,
        &merged_view,
    )?;
    let json_bytes = std::fs::read(merged_json)?;
    let csv_bytes = std::fs::read(merged_csv)?;
    let merged: serde_json::Value = serde_json::from_slice(&json_bytes)?;
    check!(eq; merged.get("athlete_id"), Some(&serde_json::json!(first_birth.public_id())));
    check!(eq; merged.get("selection"), Some(&serde_json::to_value(rows.first().ok_or("original selection")?)?));
    let Fixture {
        directory,
        store,
        writer,
        ..
    } = fixture;
    drop(writer);
    drop(store);
    let (store, _) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    let (reopened, _reopened_proofs) = capture(&store)?;
    let (json, csv) = super::super::selected::write(
        &directory.path().join("reopened"),
        &rows,
        "co2027",
        EventSemantics::ResultGroups21,
        &reopened,
    )?;
    check!(eq; std::fs::read(json)?, json_bytes);
    check!(eq; std::fs::read(csv)?, csv_bytes);
    Ok(())
}

#[test]
fn selected_bests_refuse_unknown_before_creating_or_replacing_any_artifact() -> TestResult {
    let fixture = source_births()?;
    let (view, _proofs) = capture(&fixture.store)?;
    let [first, _, unknown] = &fixture.athletes;
    let rows = [row(first), row(unknown)];
    let missing = fixture.directory.path().join("uncreated");
    let existing = fixture.directory.path().join("existing");
    std::fs::create_dir(&existing)?;
    let jsonl = existing.join("best-results-co2027.jsonl");
    let csv = existing.join("best-results-co2027.csv");
    std::fs::write(&jsonl, b"historical jsonl\n")?;
    std::fs::write(&csv, b"historical csv\n")?;
    for directory in [&missing, &existing] {
        let result = super::super::selected::write(
            directory,
            &rows,
            "co2027",
            EventSemantics::ResultGroups21,
            &view,
        );
        check!(
            matches!(result, Err(crate::report::ReportError::Store(StoreError::Invariant { detail }))
            if detail == "selected identity has no unambiguous retained public athlete ID")
        );
    }
    check!(!missing.exists());
    check!(eq; std::fs::read(jsonl)?.as_slice(), b"historical jsonl\n");
    check!(eq; std::fs::read(csv)?.as_slice(), b"historical csv\n");
    Ok(())
}

#[test]
fn historical_bests_keep_original_schema_and_candidate_id_bytes() -> TestResult {
    let fixture = crate::export::public_identity::tests::fixture::source_births()?;
    let [first, _, _] = &fixture.athletes;
    let rows = [row(first)];
    let expected_json = format!("{}\n", serde_json::to_string(rows.first().ok_or("row")?)?);
    let (jsonl, csv) = write(
        &fixture.directory.path().join("historical"),
        &rows,
        "co2027",
        EventSemantics::ResultGroups21,
    )?;
    check!(eq; std::fs::read(jsonl)?, expected_json.into_bytes());
    let mut reader = csv::Reader::from_path(csv)?;
    let header = reader.headers()?.clone();
    let record = reader.records().next().ok_or("row")??;
    check!(eq; header.len(), 39);
    check!(!header.iter().any(|name| name == "canonical_candidate_id"));
    check!(eq; record.get(column(&header, "athlete_id")?), Some(first.id.as_str()));
    Ok(())
}

#[test]
fn empty_selected_bests_keep_explicit_selected_csv_schema_and_empty_jsonl() -> TestResult {
    let fixture = source_births()?;
    let (view, _proofs) = capture(&fixture.store)?;
    let (jsonl, csv) = super::super::selected::write(
        &fixture.directory.path().join("empty"),
        &[],
        "co2027",
        EventSemantics::ResultGroups21,
        &view,
    )?;
    check!(eq; std::fs::read(jsonl)?.len(), 0);
    let mut reader = csv::Reader::from_path(csv)?;
    check!(eq; reader.headers()?.len(), 44);
    check!(eq; reader.headers()?.get(39), Some("canonical_candidate_id"));
    check!(eq; reader.records().count(), 0);
    Ok(())
}
