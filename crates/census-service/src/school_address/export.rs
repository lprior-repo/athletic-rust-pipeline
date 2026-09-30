use anyhow::{anyhow, Context, Result};
use census_domain::school_directory::SchoolDirectoryEntry;
use serde::Serialize;
use sha2::{Digest, Sha256};

const CSV_HEADER: [&str; 15] = [
    "key",
    "name",
    "kind",
    "street1",
    "street2",
    "city",
    "state",
    "zip",
    "phone",
    "website",
    "grades",
    "enrollment",
    "latitude",
    "longitude",
    "sources",
];

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub(super) fn json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(value)?)
}

pub(super) fn entries_csv(entries: &[SchoolDirectoryEntry]) -> Result<Vec<u8>> {
    let mut writer = csv::WriterBuilder::new().from_writer(Vec::new());
    writer
        .write_record(CSV_HEADER)
        .context("writing the corpus csv header")?;
    for entry in entries {
        writer
            .write_record(record(entry))
            .with_context(|| format!("writing {} to the corpus csv", entry.key().label()))?;
    }
    writer.flush().context("flushing the corpus csv")?;
    let bytes = writer
        .into_inner()
        .map_err(|error| anyhow!("finishing the corpus csv: {error}"))?;
    Ok(bytes)
}

fn record(entry: &SchoolDirectoryEntry) -> Vec<String> {
    let point = entry.coordinates();
    vec![
        entry.key().label(),
        name(entry),
        entry.kind().map(|kind| kind.label()).unwrap_or_default(),
        street(entry, true),
        street(entry, false),
        entry
            .address()
            .and_then(|address| address.city())
            .map(|city| city.as_str().to_string())
            .unwrap_or_default(),
        entry
            .address()
            .and_then(|address| address.state())
            .map(|state| state.code().to_string())
            .unwrap_or_default(),
        entry
            .address()
            .and_then(|address| address.zip())
            .map(|zip| zip.to_string())
            .unwrap_or_default(),
        entry
            .phone()
            .map(|phone| phone.as_str().to_string())
            .unwrap_or_default(),
        entry
            .website()
            .map(|website| website.as_str().to_string())
            .unwrap_or_default(),
        entry
            .grades()
            .map(|grades| grades.label())
            .unwrap_or_default(),
        entry
            .enrollment()
            .map(|count| count.get().to_string())
            .unwrap_or_default(),
        point
            .map(|point| point.latitude().to_string())
            .unwrap_or_default(),
        point
            .map(|point| point.longitude().to_string())
            .unwrap_or_default(),
        entry
            .sources()
            .iter()
            .map(|source| source.label())
            .collect::<Vec<_>>()
            .join(";"),
    ]
}

fn name(entry: &SchoolDirectoryEntry) -> String {
    entry
        .name()
        .map(|name| name.as_str().to_string())
        .unwrap_or_default()
}

fn street(entry: &SchoolDirectoryEntry, first: bool) -> String {
    entry
        .address()
        .and_then(|address| {
            if first {
                address.line1()
            } else {
                address.line2()
            }
        })
        .map(|line| line.as_str().to_string())
        .unwrap_or_default()
}
