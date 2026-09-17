use super::super::Runtime;
use crate::{
    model::SourceRecord,
    runtime::{
        protocol::ReviewInput,
        row_protocol::{RowJob, RowReport, RowResolution, ROW_PROTOCOL_REVISION},
    },
};
use anyhow::Context;
use restate_sdk::prelude::*;
use serde::Serialize;
use std::{
    io::{self, Write},
    sync::Arc,
};

pub(crate) async fn publish<T>(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    name: &'static str,
    value: T,
) -> std::result::Result<crate::domain::identity::EvidenceDigest, TerminalError>
where
    T: Serialize + Send + 'static,
{
    let digest = ctx
        .run(|| async move { runtime.store_json(value).await.map(Json).map_err(terminal) })
        .name(name)
        .retry_policy(RunRetryPolicy::new().max_attempts(1))
        .await?;
    Ok(digest.0)
}

pub(crate) async fn publish_review_input(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    input: ReviewInput,
) -> std::result::Result<crate::domain::identity::EvidenceDigest, TerminalError> {
    let store = runtime.store.clone();
    ctx.run(|| async move {
        runtime
            .blocking(move || {
                let bytes = encode_bounded(&input)?;
                store.put_bytes(&bytes).map_err(anyhow::Error::from)
            })
            .await
            .map(Json)
            .map_err(terminal)
    })
    .name("row-review-input-publication")
    .retry_policy(RunRetryPolicy::new().max_attempts(1))
    .await
    .map(|value: Json<crate::domain::identity::EvidenceDigest>| value.0)
}

pub(crate) async fn publish_report(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    report: RowReport,
) -> std::result::Result<Json<crate::domain::identity::EvidenceDigest>, HandlerError> {
    let digest = publish(ctx, runtime, "row-report-publication", report).await?;
    ctx.set("result", Json(digest.clone()));
    Ok(Json(digest))
}

pub(crate) async fn publish_terminal(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    job: RowJob,
    issues: Vec<String>,
) -> std::result::Result<Json<crate::domain::identity::EvidenceDigest>, HandlerError> {
    publish_report(
        ctx,
        runtime,
        RowReport {
            revision: ROW_PROTOCOL_REVISION.to_owned(),
            job,
            resolution: RowResolution::ReviewRequired,
            discovery: None,
            candidates: Vec::new(),
            assessment: None,
            query_evidence: Vec::new(),
            review: None,
            issues,
        },
    )
    .await
}

pub(crate) fn validate_job(ctx: &ObjectContext<'_>, job: &RowJob) -> Result<(), HandlerError> {
    let key = job.key().map_err(terminal)?;
    if key == ctx.key() {
        Ok(())
    } else {
        Err(terminal(
            "row worker key does not bind workbook, snapshot, and source row",
        ))
    }
}

pub(crate) fn source_validation(source: &SourceRecord) -> Option<String> {
    let first = field(source, &["Person First", "first_name"]);
    let last = field(source, &["Person Last", "last_name"]);
    if first.is_none() || last.is_none() {
        return Some(
            "query plan validation failed: source row has no complete athlete name".to_owned(),
        );
    }
    None
}

fn field<'a>(source: &'a SourceRecord, names: &[&str]) -> Option<&'a str> {
    names
        .iter()
        .find_map(|name| source.fields.get(*name))
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(crate) fn encode_bounded<T: Serialize>(value: &T) -> anyhow::Result<Vec<u8>> {
    let mut writer = BoundedJson(Vec::with_capacity(65_536));
    serde_json::to_writer(&mut writer, value).context("encoding bounded review input")?;
    Ok(writer.0)
}

struct BoundedJson(Vec<u8>);
impl Write for BoundedJson {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        const LIMIT: usize = 65_536;
        let size = self
            .0
            .len()
            .checked_add(bytes.len())
            .filter(|size| *size <= LIMIT)
            .ok_or_else(|| io::Error::other("review input exceeds 64 KiB"))?;
        self.0
            .try_reserve(size.saturating_sub(self.0.len()))
            .map_err(io::Error::other)?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}

#[cfg(test)]
mod tests {
    use super::source_validation;
    use crate::model::SourceRecord;
    use std::collections::BTreeMap;

    fn source(fields: &[(&str, &str)]) -> SourceRecord {
        SourceRecord {
            fields: fields
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect::<BTreeMap<_, _>>(),
            ..SourceRecord::default()
        }
    }

    #[test]
    fn missing_name_is_terminal_review_input() {
        let issue =
            source_validation(&source(&[("Schools Name", "Central")])).expect("missing name issue");
        assert!(issue.contains("complete athlete name"));
    }
}
