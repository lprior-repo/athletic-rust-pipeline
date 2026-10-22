use super::rows::{csv, fields, json, keyed, read, sheet};
use super::{Expected, TestResult};
use calamine::{open_workbook, Reader, Xlsx};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn artifacts(directory: &Path, workbook: &Path, expected: &Expected) -> TestResult {
    inventory(directory)?;
    check!(csv(&directory.join("recruiting.csv"))?.is_empty());
    empty_bests(directory)?;
    affiliation_rows(directory, expected)?;
    aliases(directory, expected)?;
    let schools = keyed(
        csv(&directory.join("schools.csv"))?,
        "school_id",
        &expected.schools.keys().cloned().collect(),
    )?;
    for (id, name) in &expected.schools {
        fields(
            schools.get(id).ok_or("retained school")?,
            &[("canonical_name", name)],
        )?;
    }
    let frozen = json(&directory.join("frozen-input.json"))?;
    for (name, original) in &expected.retained {
        check!(eq; frozen.pointer(&format!("/data/{name}")), Some(original));
    }
    check!(eq; json(&directory.join("census-all-sources.json"))?.pointer("/totals/athletes").and_then(Value::as_u64), Some(0));
    workbook_rows(workbook, expected)
}

fn inventory(directory: &Path) -> TestResult {
    let manifest = json(&directory.join("manifest.json"))?;
    let counts = manifest
        .get("normalized_csv_rows")
        .and_then(Value::as_object)
        .ok_or("normalized inventory")?;
    check!(eq; counts.len(), 11);
    for (name, expected) in [
        ("athletes.csv", 0),
        ("best_marks.csv", 0),
        ("schools.csv", 2),
        ("coach_contacts.csv", 0),
        ("comparison_classes.csv", 0),
        ("provenance.csv", 0),
        ("coverage.csv", 0),
        ("identity_aliases.csv", 5),
        ("combined_exclusions.csv", 0),
        ("current_affiliations.csv", 5),
        ("participation_exclusions.csv", 0),
    ] {
        check!(eq; counts.get(name).and_then(Value::as_u64), Some(expected));
        check!(eq; u64::try_from(csv(&directory.join(name))?.len())?, expected);
    }
    Ok(())
}

fn empty_bests(directory: &Path) -> TestResult {
    let path = directory.join("best-results-co2027.csv");
    check!(csv(&path)?.is_empty());
    let contents = read(&path)?;
    let mut reader = csv::Reader::from_reader(contents.as_slice());
    let headers = reader.headers()?;
    for name in [
        "athlete_id",
        "canonical_candidate_id",
        "current_school_id",
        "current_state",
        "school_at_performance",
        "performance_id",
    ] {
        check!(headers.iter().any(|column| column == name));
    }
    check!(read(&directory.join("best-results-co2027.jsonl"))?.is_empty());
    Ok(())
}

fn candidates(expected: &Expected) -> BTreeSet<String> {
    expected
        .roster
        .iter()
        .cloned()
        .chain([
            expected.first.clone(),
            expected.second.clone(),
            expected.unknown.clone(),
        ])
        .collect()
}

fn affiliation_rows(directory: &Path, expected: &Expected) -> TestResult {
    let rows = keyed(
        csv(&directory.join("current_affiliations.csv"))?,
        "retained_candidate_id",
        &candidates(expected),
    )?;
    let references = serde_json::to_string(&expected.references)?;
    for (candidate, row) in &rows {
        check!(eq; row.len(), 8);
        let (status, refs, reasons) = if candidate == &expected.first {
            (
                "conflicting_affiliation_evidence",
                references.as_str(),
                "[\"no_retained_row\"]",
            )
        } else if candidate == &expected.second {
            ("current_affiliation_unknown", "[]", "[\"no_retained_row\"]")
        } else if candidate == &expected.unknown {
            (
                "current_affiliation_unknown",
                "[]",
                "[\"identity_unaccepted\"]",
            )
        } else {
            (
                "current_affiliation_unknown",
                "[]",
                "[\"missing_current_affiliation_evidence\"]",
            )
        };
        fields(
            row,
            &[
                ("current_school_id", ""),
                ("resolution_status", status),
                ("evidence_refs", refs),
                ("unavailable_reasons", reasons),
                ("profile_id", &expected.profile),
                ("as_of", "2026-10-01"),
                ("input_generation", &expected.generation),
            ],
        )?;
    }
    Ok(())
}

fn aliases(directory: &Path, expected: &Expected) -> TestResult {
    let rows = keyed(
        csv(&directory.join("identity_aliases.csv"))?,
        "original_id",
        &candidates(expected),
    )?;
    for (candidate, row) in &rows {
        let (canonical, public, state) = if candidate == &expected.unknown {
            (expected.unknown.as_str(), "", "unknown")
        } else if candidate == &expected.second {
            (
                expected.second.as_str(),
                expected.second_public.as_str(),
                "canonical",
            )
        } else {
            (
                expected.first.as_str(),
                expected.first_public.as_str(),
                "canonical",
            )
        };
        fields(
            row,
            &[
                ("canonical_id", "unavailable"),
                ("resolved_id", canonical),
                ("public_athlete_id", public),
                ("publication_membership", "canonical_outside_publication"),
                ("publication_identity_status", state),
                ("input_generation", &expected.generation),
            ],
        )?;
        let resolution: Value =
            serde_json::from_str(row.get("identity_resolution").ok_or("resolution audit")?)?;
        check!(eq; resolution.get("state").and_then(Value::as_str), Some(state));
        if candidate != &expected.unknown {
            check!(eq; resolution.pointer("/successor/public_id").and_then(Value::as_str), Some(public));
            check!(eq; resolution.pointer("/successor/representative_candidate").and_then(Value::as_str), Some(canonical));
        }
    }
    Ok(())
}

fn workbook_rows(path: &Path, expected: &Expected) -> TestResult {
    check!(std::fs::metadata(path)?.len() <= 16 * 1024 * 1024);
    let mut book: Xlsx<_> = open_workbook(path)?;
    check!(eq; book.sheet_names(), vec!["Athletes".to_owned()]);
    check!(sheet(&mut book, "Athletes")?.is_empty());
    let directory = path.parent().ok_or("publication generation")?;
    let aliases = csv(&directory.join("identity_aliases.csv"))?;
    check!(eq; aliases.len(), 5);
    let accepted = aliases
        .iter()
        .filter_map(|row| row.get("public_athlete_id"))
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    check!(eq; accepted.len(), 4);
    check!(eq; accepted.into_iter().collect::<BTreeSet<_>>().len(), 2);
    let withheld = aliases
        .iter()
        .filter(|row| row.get("original_id") == Some(&expected.unknown))
        .collect::<Vec<_>>();
    check!(eq; withheld.len(), 1);
    check!(eq; withheld.first().and_then(|row| row.get("publication_identity_status")).map(String::as_str), Some("unknown"));
    let manifest = json(&directory.join("manifest.json"))?;
    let digest = manifest
        .pointer("/artifacts/frozen-input.json/sha256")
        .and_then(Value::as_str)
        .ok_or("frozen commitment")?;
    let dataset = crate::export::ExportDataset::reopen_frozen_with_digest(
        &directory.join("frozen-input.json"),
        digest,
    )?;
    let report = crate::report::coverage_report(&dataset, Some(2027))?;
    let prefix = "retained current affiliation exclusions: ";
    let notes = report
        .notes
        .into_iter()
        .filter(|note| note.starts_with(prefix))
        .collect::<BTreeSet<_>>();
    check!(eq; notes, BTreeSet::from([
        format!("{prefix}reason=current_affiliation_unresolved status=conflicting_affiliation_evidence athletes=1"),
        format!("{prefix}reason=current_affiliation_unresolved status=current_affiliation_unknown athletes=1"),
    ]));
    Ok(())
}
