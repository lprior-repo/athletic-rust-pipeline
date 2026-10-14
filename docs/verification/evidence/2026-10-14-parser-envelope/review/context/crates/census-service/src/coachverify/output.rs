use anyhow::Context;
use census_domain::model::{ContactClaimEvidence, RawContactRow, CONTACT_COLUMNS};
use std::path::Path;

use super::fetch::verify_one_fragment;
use super::limits::{self, LimitedReader, LimitedWriter};

pub fn read_fragment(path: &Path) -> anyhow::Result<Vec<RawContactRow>> {
    let mut rows = Vec::new();
    for_each_fragment_row(path, |row| {
        rows.try_reserve(1)?;
        rows.push(row);
        Ok(())
    })?;
    Ok(rows)
}

pub(super) fn for_each_fragment_row(
    path: &Path,
    mut visit: impl FnMut(RawContactRow) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let file = std::fs::File::open(path).with_context(|| format!("open fragment {path:?}"))?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(LimitedReader::new(file));
    let mut count = 0usize;
    for record in reader.records() {
        let record = record.with_context(|| format!("read fragment row in {path:?}"))?;
        limits::check(
            "contact record bytes",
            record.as_slice().len(),
            limits::MAX_RECORD_BYTES,
        )?;
        if record.iter().next() == Some("school") && record.get(2) == Some("state") {
            continue;
        }
        count = count
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("contact row count overflow"))?;
        limits::check("contact rows", count, limits::MAX_ROWS)?;
        visit(contact_row(&record))?;
    }
    Ok(())
}

fn contact_row(record: &csv::StringRecord) -> RawContactRow {
    let cell = |index: usize| {
        record
            .get(index)
            .map_or("", core::convert::identity)
            .trim()
            .to_string()
    };
    RawContactRow {
        school: cell(0),
        city: cell(1),
        state: cell(2),
        sport: cell(3),
        role: cell(4),
        coach_name: cell(5),
        public_professional_email: cell(6),
        ad_name: cell(7),
        ad_email: cell(8),
        source_urls: cell(9).split_whitespace().map(str::to_string).collect(),
        last_observed: cell(10),
    }
}

pub fn read_evidence_jsonl(path: &Path) -> anyhow::Result<Vec<ContactClaimEvidence>> {
    let file = std::fs::File::open(path).with_context(|| format!("open evidence {path:?}"))?;
    #[cfg(test)]
    metrics::opened();
    let mut reader = std::io::BufReader::new(LimitedReader::new(file));
    let mut buffer = Vec::new();
    let mut claims = Vec::new();
    while limits::line(&mut reader, &mut buffer)
        .with_context(|| format!("read evidence from {path:?}"))?
    {
        limits::check(
            "contact claims",
            claims.len().saturating_add(1),
            limits::MAX_ROWS,
        )?;
        let claim = serde_json::from_slice(&buffer)
            .with_context(|| format!("parse evidence from {path:?}"))?;
        claims.try_reserve(1)?;
        claims.push(claim);
    }
    Ok(claims)
}

pub fn write_evidence_jsonl<'a>(
    path: &Path,
    claims: impl IntoIterator<Item = &'a ContactClaimEvidence>,
) -> anyhow::Result<()> {
    use std::io::Write;
    census_store::read::publish_atomically(path, |temporary| {
        let file =
            std::fs::File::create(temporary).map_err(|source| census_store::StoreError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        let mut writer = LimitedWriter::new(std::io::BufWriter::new(file));
        let mut count = 0usize;
        for claim in claims {
            limits::claim_bytes(claim).map_err(|error| census_store::StoreError::Invariant {
                detail: error.to_string(),
            })?;
            count = count
                .checked_add(1)
                .ok_or_else(|| census_store::StoreError::Invariant {
                    detail: "contact claim count overflow".to_string(),
                })?;
            limits::check("contact claims", count, limits::MAX_ROWS).map_err(|error| {
                census_store::StoreError::Invariant {
                    detail: error.to_string(),
                }
            })?;
            serde_json::to_writer(&mut writer, claim).map_err(|error| {
                census_store::StoreError::Invariant {
                    detail: error.to_string(),
                }
            })?;
            writer
                .write_all(b"\n")
                .map_err(|source| census_store::StoreError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
        }
        writer
            .flush()
            .map_err(|source| census_store::StoreError::Io {
                path: path.to_path_buf(),
                source,
            })
    })?;
    Ok(())
}

pub fn write_fragment(path: &Path, outcomes: &[super::RowOutcome]) -> anyhow::Result<()> {
    limits::check("contact rows", outcomes.len(), limits::MAX_ROWS)?;
    for outcome in outcomes {
        limits::row_bytes(&outcome.row)?;
    }
    write_evidence_jsonl(
        &evidence_path(path),
        outcomes.iter().flat_map(|outcome| &outcome.evidence),
    )?;
    publish_fragment(path, outcomes)
}

fn publish_fragment(path: &Path, outcomes: &[super::RowOutcome]) -> anyhow::Result<()> {
    census_store::read::publish_atomically(path, |temporary| {
        let file =
            std::fs::File::create(temporary).map_err(|source| census_store::StoreError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        let mut writer = csv::WriterBuilder::new().from_writer(LimitedWriter::new(file));
        writer
            .write_record(CONTACT_COLUMNS)
            .map_err(|error| census_store::read::csv_failure(path, error))?;
        for outcome in outcomes.iter().filter(|outcome| outcome.verdict.shipped()) {
            let row = &outcome.row;
            writer
                .write_record([
                    &row.school,
                    &row.city,
                    &row.state,
                    &row.sport,
                    &row.role,
                    &row.coach_name,
                    &row.public_professional_email,
                    &row.ad_name,
                    &row.ad_email,
                    &row.source_urls.join(" "),
                    &row.last_observed,
                ])
                .map_err(|error| census_store::read::csv_failure(path, error))?;
        }
        writer
            .flush()
            .map_err(|source| census_store::StoreError::Io {
                path: path.to_path_buf(),
                source,
            })
    })?;
    Ok(())
}

pub fn evidence_path(path: &Path) -> std::path::PathBuf {
    let mut name = match path.file_name().map(std::ffi::OsStr::to_os_string) {
        Some(value) => value,
        None => std::ffi::OsString::from("contacts.csv"),
    };
    name.push(".evidence.jsonl");
    path.with_file_name(name)
}

pub fn fragment_file_name(path: &Path) -> String {
    let file = match path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
    {
        Some(value) => value,
        None => "contacts.csv".to_string(),
    };
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

#[cfg(test)]
pub(super) mod metrics {
    std::thread_local! {
        pub static OPENS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        pub static COMPARISONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
    pub fn opened() {
        OPENS.with(|count| count.set(count.get().saturating_add(1)));
    }
    pub fn comparison() {
        COMPARISONS.with(|count| count.set(count.get().saturating_add(1)));
    }
}
