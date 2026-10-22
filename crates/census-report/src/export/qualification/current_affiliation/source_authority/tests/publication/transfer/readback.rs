use super::*;
use calamine::{open_workbook, Reader, Xlsx};
use std::collections::{BTreeMap, BTreeSet};

type Row = BTreeMap<String, String>;

pub(super) fn artifacts(
    path: &Path,
    dataset: &ExportDataset,
    transfer: &Transfer,
) -> TestResult<Value> {
    read(
        path,
        dataset,
        &transfer.old,
        &transfer.current,
        &transfer.address_digest,
        transfer.case,
    )
}
pub(super) fn retained_artifacts(
    path: &Path,
    dataset: &ExportDataset,
    retained: &Retained,
) -> TestResult<Value> {
    read(
        path,
        dataset,
        &retained.old,
        &retained.current,
        &retained.address_digest,
        retained.case,
    )
}

fn read(
    path: &Path,
    dataset: &ExportDataset,
    old: &fixture::Capture,
    current: &fixture::Capture,
    address: &str,
    case: Case,
) -> TestResult<Value> {
    let directory = path.parent().ok_or("controlled transfer generation")?;
    let athletes = csv(&directory.join("athletes.csv"))?;
    let recruiting = csv(&directory.join("recruiting.csv"))?;
    let schools = csv(&directory.join("schools.csv"))?;
    let affiliations = csv(&directory.join("current_affiliations.csv"))?;
    let aliases = csv(&directory.join("identity_aliases.csv"))?;
    let exclusions = csv(&directory.join("participation_exclusions.csv"))?;
    let mut book: Xlsx<_> = open_workbook(path)?;
    let athlete_sheet = sheet(&mut book, "Athletes")?;
    check!(eq; book.sheet_names(), vec!["Athletes".to_owned()]);
    let included = matches!(case, Case::Current);
    let count = usize::from(included);
    check!(eq; athletes.len(),count);
    check!(eq; recruiting.len(),count);
    check!(eq; athlete_sheet.len(),count);
    check!(eq; affiliations.len(),2);
    check!(eq; aliases.len(),2);
    check!(csv(&directory.join("best_marks.csv"))?.is_empty());
    let identities = dataset.verified_public_identities()?;
    let public = identities
        .public_id(old.athlete.id.as_str())
        .ok_or("controlled transfer public id")?;
    for candidate in [&old.athlete.id, &current.athlete.id] {
        let alias = unique(&aliases, "original_id", candidate.as_str())?;
        fields(alias, &[("public_athlete_id", public.as_str())])?;
    }
    let resolution = dataset
        .verified_current_affiliations()?
        .resolution(&old.athlete.id);
    fields(
        unique(
            &affiliations,
            "retained_candidate_id",
            old.athlete.id.as_str(),
        )?,
        &[
            ("resolution_status", resolution.status()),
            (
                "current_school_id",
                resolution.school().map_or("", SchoolId::as_str),
            ),
        ],
    )?;
    let performance = dataset
        .performances
        .first()
        .ok_or("historical performance")?;
    if included {
        fields(
            athletes.first().ok_or("nonempty current athlete")?,
            &[
                ("athlete_id", public.as_str()),
                ("canonical_candidate_id", old.athlete.id.as_str()),
                ("current_school_id", current.school.id.as_str()),
            ],
        )?;
        fields(
            recruiting.first().ok_or("nonempty recruiting transfer")?,
            &[
                ("school", &current.school.name),
                ("state", "MI"),
                (
                    "school_address",
                    "123 Controlled Transfer Lane, Novi, MI 48375",
                ),
            ],
        )?;
        fields(
            athlete_sheet.first().ok_or("nonempty workbook transfer")?,
            &[
                ("Athlete ID", public.as_str()),
                ("School ID", current.school.id.as_str()),
                ("School State", "MI"),
                ("Observed bests", ""),
                (
                    "School Address",
                    "123 Controlled Transfer Lane, Novi, MI 48375",
                ),
            ],
        )?;
        fields(
            unique(&schools, "school_id", current.school.id.as_str())?,
            &[
                ("postal_school_id", current.school.id.as_str()),
                ("postal_street", "123 Controlled Transfer Lane"),
                ("postal_state", "MI"),
                (
                    "postal_owner_namespace",
                    "association_school:controlled-transfer-association",
                ),
                ("postal_owner_id", "current-transfer"),
                (
                    "postal_source_url",
                    "https://controlled-association.test/schools/current-transfer",
                ),
                ("postal_capture_sha256", address),
            ],
        )?;
        check!(eq; performance.athlete, old.athlete.id);
        check!(eq; performance.date, "2026-05-01");
        check!(eq; performance.source_key, "old-result");
        check!(eq; dataset.teams.get(&performance.team).ok_or("historical team")?.school, old.school.id);
        let retained_school = dataset
            .schools
            .get(&current.school.id)
            .ok_or("retained current postal school")?;
        check!(eq; retained_school.postal_addresses, current.school.postal_addresses);
        check!(eq; exclusions.len(),1);
    }
    let frozen: Value =
        serde_json::from_slice(&std::fs::read(directory.join("frozen-input.json"))?)?;
    check!(eq; frozen.pointer("/data/performances"),Some(&serde_json::to_value(&dataset.performances)?));
    check!(eq; frozen.pointer("/data/teams"),Some(&serde_json::to_value(&dataset.teams)?));
    Ok(
        json!({"athletes":athletes,"recruiting":recruiting,"schools":schools,"affiliations":affiliations,"aliases":aliases,"exclusions":exclusions,"athlete_sheet":athlete_sheet}),
    )
}

fn unique<'a>(rows: &'a [Row], key: &str, value: &str) -> TestResult<&'a Row> {
    let mut selected = rows
        .iter()
        .filter(|row| row.get(key).map(String::as_str) == Some(value));
    let row = selected.next().ok_or("controlled output row")?;
    check!(selected.next().is_none());
    Ok(row)
}
fn fields(row: &Row, expected: &[(&str, &str)]) -> TestResult {
    for (key, value) in expected {
        check!(eq;row.get(*key).map(String::as_str),Some(*value),"{key}");
    }
    Ok(())
}
fn csv(path: &Path) -> TestResult<Vec<Row>> {
    check!(std::fs::metadata(path)?.len() <= 8 * 1024 * 1024);
    let mut reader = csv::Reader::from_path(path)?;
    let header = reader.headers()?.clone();
    check!(eq;header.iter().collect::<BTreeSet<_>>().len(),header.len());
    let mut rows = Vec::new();
    for record in reader.records() {
        check!(rows.len() < 128);
        let record = record?;
        check!(eq;record.len(),header.len());
        rows.push(
            header
                .iter()
                .zip(record.iter())
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect(),
        );
    }
    Ok(rows)
}
fn sheet(book: &mut Xlsx<std::io::BufReader<std::fs::File>>, name: &str) -> TestResult<Vec<Row>> {
    let range = book.worksheet_range(name)?;
    check!(range.height() <= 128 && range.width() <= 128);
    let mut rows = range.rows();
    let headers = rows
        .next()
        .ok_or("controlled worksheet header")?
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    check!(eq;headers.iter().collect::<BTreeSet<_>>().len(),headers.len());
    rows.map(|row| {
        check!(eq;row.len(),headers.len());
        Ok(headers
            .iter()
            .cloned()
            .zip(row.iter().map(ToString::to_string))
            .collect())
    })
    .collect()
}
