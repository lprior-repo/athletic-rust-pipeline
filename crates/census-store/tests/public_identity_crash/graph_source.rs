use std::collections::{BTreeMap, BTreeSet};

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Value};

use super::{digest, Rows, TestResult};

const TABLES: [&str; 6] = [
    "athletes",
    "source_observations",
    "review_cases",
    "identity_verdicts",
    "performances",
    "relay_results",
];
const FIXTURE_ROWS: usize = 2048;
const FIXTURE_VALUE_BYTES: usize = 65_536;

pub(super) struct SourceImage {
    expected: BTreeMap<&'static str, Rows>,
    seen: BTreeMap<&'static str, Rows>,
    tables: BTreeSet<String>,
    derived_generation: u64,
}

impl SourceImage {
    pub(super) fn capture(entities: &Rows, meta: &Rows) -> TestResult<Self> {
        check!(entities.len() <= FIXTURE_ROWS);
        let raw = meta
            .get(b"generation:derived:current".as_slice())
            .ok_or("source derived generation")?;
        let text = std::str::from_utf8(raw)?;
        let derived_generation = text.parse::<u64>()?;
        check!(eq; text, derived_generation.to_string());
        let mut expected = BTreeMap::new();
        let mut seen = BTreeMap::new();
        for table in TABLES {
            let prefix = source_prefix(table, derived_generation);
            let mut rows = Rows::new();
            for (key, value) in entities.iter().filter(|(key, _)| key.starts_with(&prefix)) {
                check!(key.len() <= 1024 && value.len() <= FIXTURE_VALUE_BYTES);
                check!(rows.insert(key.clone(), value.clone()).is_none());
            }
            check!(expected.insert(table, rows).is_none());
            check!(seen.insert(table, Rows::new()).is_none());
        }
        Ok(Self {
            expected,
            seen,
            tables: BTreeSet::new(),
            derived_generation,
        })
    }

    pub(super) fn derived_generation(&self) -> u64 {
        self.derived_generation
    }

    pub(super) fn observe_object(&mut self, rows: &Rows) -> TestResult {
        if !rows.keys().any(|key| key.starts_with(b"j2\0manifest\0")) {
            return Ok(());
        }
        let mut bytes = Vec::new();
        for (_, raw) in rows
            .iter()
            .filter(|(key, _)| key.starts_with(b"j2\0chunk\0"))
        {
            let chunk: Value = serde_json::from_slice(raw)?;
            let decoded = BASE64_STANDARD.decode(chunk["data"].as_str().ok_or("graph chunk")?)?;
            check!(
                bytes
                    .len()
                    .checked_add(decoded.len())
                    .ok_or("source object length")?
                    <= 25_165_824
            );
            bytes.extend_from_slice(&decoded);
        }
        let records = bytes
            .split_inclusive(|byte| *byte == b'\n')
            .map(serde_json::from_slice::<Value>)
            .collect::<Result<Vec<_>, _>>()?;
        let header = &records.first().ok_or("source object header")?["payload"]["value"];
        match header["schema"].as_str().ok_or("source object schema")? {
            "public-identity-authority-table/v2" => {
                let table = header["binding"]["table"].as_str().ok_or("source table")?;
                check!(self.expected.contains_key(table), "unexpected source table");
                check!(
                    self.tables.insert(table.to_owned()),
                    "duplicate source table manifest"
                );
            }
            "public-identity-authority-raw-page/v2" => self.observe_page(header, &records)?,
            "public-identity-authority-page-refs/v2"
            | "public-identity-authority-generation-seal/v2" => (),
            _ => return Err("unexpected graph object schema".into()),
        }
        Ok(())
    }

    fn observe_page(&mut self, header: &Value, records: &[Value]) -> TestResult {
        let table = header["table"].as_str().ok_or("raw page table")?;
        check!(
            self.expected.contains_key(table),
            "unexpected raw page table"
        );
        check!(records.len() <= 1537);
        let mut index = 1usize;
        let mut count = 0usize;
        while index < records.len() {
            check!(count < 512);
            let row = &records[index];
            check!(eq; row["payload"]["kind"], json!("raw_row"));
            let value = &row["payload"]["value"];
            let key =
                BASE64_STANDARD.decode(value["physical_key_base64"].as_str().ok_or("raw key")?)?;
            let parts = usize::try_from(value["parts"].as_u64().ok_or("raw parts")?)?;
            check!(parts > 0 && parts <= 128);
            let mut raw = Vec::new();
            for part in 0..parts {
                let ordinal = index
                    .checked_add(part)
                    .and_then(|ordinal| ordinal.checked_add(1))
                    .ok_or("part ordinal")?;
                let fragment = records.get(ordinal).ok_or("raw part")?;
                check!(eq; fragment["payload"]["kind"], json!("raw_part"));
                let decoded = BASE64_STANDARD.decode(
                    fragment["payload"]["value"]["bytes_base64"]
                        .as_str()
                        .ok_or("raw bytes")?,
                )?;
                check!(
                    raw.len()
                        .checked_add(decoded.len())
                        .ok_or("source row length")?
                        <= FIXTURE_VALUE_BYTES
                );
                raw.extend_from_slice(&decoded);
            }
            check!(eq; value["value_bytes"], json!(raw.len()));
            check!(eq; value["value_sha256"], json!(digest(&raw)));
            let expected = self.expected.get(table).ok_or("expected table")?;
            check!(eq; expected.get(&key), Some(&raw));
            let seen = self.seen.get_mut(table).ok_or("seen table")?;
            check!(
                seen.insert(key, raw).is_none(),
                "duplicate physical source key"
            );
            count = count.checked_add(1).ok_or("page row count")?;
            index = index
                .checked_add(parts)
                .and_then(|ordinal| ordinal.checked_add(1))
                .ok_or("record ordinal")?;
        }
        check!(eq; header["rows"], json!(count));
        Ok(())
    }

    pub(super) fn finish(self, complete: bool) -> TestResult {
        if complete {
            let expected_tables = TABLES
                .into_iter()
                .map(str::to_owned)
                .collect::<BTreeSet<_>>();
            check!(eq; self.tables, expected_tables, "complete six-table manifest inventory");
            check!(eq; self.seen, self.expected, "complete physical source inventory");
        }
        Ok(())
    }
}

fn source_prefix(table: &str, derived_generation: u64) -> Vec<u8> {
    if table == "review_cases" {
        let mut prefix = b"derived\0review_cases\0".to_vec();
        prefix.extend_from_slice(&derived_generation.to_be_bytes());
        prefix.push(0);
        prefix
    } else {
        format!("{table}\0").into_bytes()
    }
}

#[path = "graph_source_tests.rs"]
mod tests;
