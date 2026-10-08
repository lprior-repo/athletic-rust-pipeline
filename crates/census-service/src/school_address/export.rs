use anyhow::{anyhow, Context, Result};
use census_domain::school_directory::{AddressKind, SchoolDirectoryEntry};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::borrow::Cow;

const CSV_HEADER: [&str; 16] = [
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
    "address_kind",
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
    entries.iter().try_for_each(|entry| {
        writer
            .write_record(record(entry).iter().map(|field| field.as_bytes()))
            .with_context(|| format!("writing {} to the corpus csv", entry.key().label()))
    })?;
    writer.flush().context("flushing the corpus csv")?;
    let bytes = writer
        .into_inner()
        .map_err(|error| anyhow!("finishing the corpus csv: {error}"))?;
    Ok(bytes)
}

fn record(entry: &SchoolDirectoryEntry) -> [Cow<'_, str>; 16] {
    let [street1, street2, city, state, zip] = address_fields(entry);
    let [phone, website, grades, enrollment] = contact_fields(entry);
    let point = entry.coordinates();
    [
        Cow::Owned(entry.key().label()),
        Cow::Borrowed(entry.name().map_or("", |name| name.as_str())),
        encoded(entry.kind().map(|kind| kind.label())),
        street1,
        street2,
        city,
        state,
        zip,
        phone,
        website,
        grades,
        enrollment,
        encoded(point.map(|point| point.latitude().to_string())),
        encoded(point.map(|point| point.longitude().to_string())),
        Cow::Owned(sources(entry)),
        Cow::Borrowed(address_kind(entry)),
    ]
}

fn address_fields(entry: &SchoolDirectoryEntry) -> [Cow<'_, str>; 5] {
    let address = entry.address();
    [
        Cow::Borrowed(
            address
                .and_then(|value| value.line1())
                .map_or("", |value| value.as_str()),
        ),
        Cow::Borrowed(
            address
                .and_then(|value| value.line2())
                .map_or("", |value| value.as_str()),
        ),
        Cow::Borrowed(
            address
                .and_then(|value| value.city())
                .map_or("", |value| value.as_str()),
        ),
        Cow::Borrowed(
            address
                .and_then(|value| value.state())
                .map_or("", |value| value.code()),
        ),
        encoded(
            address
                .and_then(|value| value.zip())
                .map(ToString::to_string),
        ),
    ]
}

fn contact_fields(entry: &SchoolDirectoryEntry) -> [Cow<'_, str>; 4] {
    [
        Cow::Borrowed(entry.phone().map_or("", |value| value.as_str())),
        Cow::Borrowed(entry.website().map_or("", |value| value.as_str())),
        encoded(entry.grades().map(|value| value.label())),
        encoded(entry.enrollment().map(|value| value.get().to_string())),
    ]
}

fn encoded(value: Option<String>) -> Cow<'static, str> {
    value.map_or(Cow::Borrowed(""), Cow::Owned)
}

fn sources(entry: &SchoolDirectoryEntry) -> String {
    entry
        .sources()
        .iter()
        .fold(String::new(), |mut text, source| {
            if !text.is_empty() {
                text.push(';');
            }
            text.push_str(&source.label());
            text
        })
}

fn address_kind(entry: &SchoolDirectoryEntry) -> &'static str {
    match entry.address().map(|address| address.kind()) {
        Some(AddressKind::Physical) => "physical",
        Some(AddressKind::Mailing) => "mailing",
        Some(AddressKind::Unknown) => "unknown",
        None => "",
    }
}
