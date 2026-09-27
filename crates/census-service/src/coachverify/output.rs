use census_domain::model::{CONTACT_COLUMNS, ContactClaimEvidence, RawContactRow};
use std::path::Path;

use super::fetch::verify_one_fragment;

pub fn read_fragment(path: &Path) -> anyhow::Result<Vec<RawContactRow>> {
    use anyhow::Context;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)
        .with_context(|| format!("open fragment {path:?}"))?;
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.with_context(|| format!("read fragment row in {path:?}"))?;
        if record.iter().next() == Some("school") && record.get(2) == Some("state") {
            continue;
        }
        let cell = |index: usize| record.get(index).unwrap_or_default().trim().to_string();
        let source_urls: Vec<String> = cell(9).split_whitespace().map(str::to_string).collect();
        rows.push(RawContactRow {
            school: cell(0),
            city: cell(1),
            state: cell(2),
            sport: cell(3),
            role: cell(4),
            coach_name: cell(5),
            public_professional_email: cell(6),
            ad_name: cell(7),
            ad_email: cell(8),
            source_urls,
            last_observed: cell(10),
        });
    }
    Ok(rows)
}

fn read_evidence_jsonl(path: &Path) -> anyhow::Result<Vec<ContactClaimEvidence>> {
    use anyhow::Context;
    use std::io::BufRead;
    let file = std::fs::File::open(path).with_context(|| format!("open evidence {path:?}"))?;
    let reader = std::io::BufReader::new(file);
    let mut claims = Vec::new();
    for line in reader.lines() {
        let line = line.with_context(|| format!("read evidence line from {path:?}"))?;
        let claim: ContactClaimEvidence = serde_json::from_str(&line)
            .with_context(|| format!("parse evidence from {path:?}"))?;
        claims.push(claim);
    }
    Ok(claims)
}

pub fn write_fragment(path: &Path, outcomes: &[super::RowOutcome]) -> anyhow::Result<()> {
    use census_store::read::publish_atomically;
    Ok(publish_atomically(path, |temporary| {
        let mut writer = csv::WriterBuilder::new()
            .from_path(temporary)
            .map_err(|error| census_store::read::csv_failure(path, error))?;
        writer
            .write_record(CONTACT_COLUMNS)
            .map_err(|error| census_store::read::csv_failure(path, error))?;
        for outcome in outcomes.iter().filter(|outcome| outcome.verdict.shipped()) {
            let row = &outcome.row;
            writer
                .write_record([
                    &row.school, &row.city, &row.state, &row.sport, &row.role,
                    &row.coach_name, &row.public_professional_email, &row.ad_name,
                    &row.ad_email, &row.source_urls.join(" "), &row.last_observed,
                ])
                .map_err(|error| census_store::read::csv_failure(path, error))?;
        }
        writer
            .flush()
            .map_err(|source| census_store::StoreError::Io {
                path: path.to_path_buf(),
                source,
            })
    })?)
}

pub fn read_fragment_evidence(path: &Path, row: &RawContactRow) -> anyhow::Result<Vec<ContactClaimEvidence>> {
    let file_name = fragment_file_name(path);
    let evidence_path = path.with_file_name(format!("{}.evidence.jsonl", file_name));
    let claims = read_evidence_jsonl(&evidence_path)?;
    Ok(claims.into_iter().filter(|c| {
        c.school == row.school && c.role == row.role && c.person == row.coach_name
    }).collect())
}

pub fn fragment_file_name(path: &Path) -> String {
    let file = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "contacts.csv".to_string());
    match path
        .parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
    {
        Some(directory) if !directory.is_empty() => format!("{directory}-{file}"),
        _ => file,
    }
}

pub async fn verify_fragment(
    fetcher: &census_crawl::net::Fetcher,
    path: &Path,
    out_dir: &Path,
    options: &super::GateOptions,
) -> anyhow::Result<super::verdict::FragmentOutcome> {
    verify_one_fragment(fetcher, path, out_dir, options).await
}
