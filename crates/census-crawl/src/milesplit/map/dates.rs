use super::{OwnedPerformance, OwnedResultSet, RowWriter, SourceContext};
use crate::context::{assess_performance_date, PerformanceDateAssessment};
use crate::{CrawlError, CrawlResult};

pub(super) fn canonical_date(published: &str) -> String {
    match crate::context::published_performance_date(published) {
        Some(date) => date.format("%Y-%m-%d").to_string(),
        None => published.to_string(),
    }
}

pub(super) fn validate(
    writer: &mut RowWriter<'_>,
    context: &SourceContext<'_>,
    owned: &OwnedResultSet<'_>,
) -> CrawlResult<()> {
    let assessment = assess_performance_date(context.performance_as_of, &context.page.meet.date);
    if !matches!(assessment, PerformanceDateAssessment::Unknown) {
        return Ok(());
    }
    owned.indices.iter().try_for_each(|index| {
        let row = owned
            .page
            .rows
            .get(*index)
            .ok_or_else(|| CrawlError::Invariant {
                detail: "owned result-set index does not address a retained source row".into(),
            })?;
        super::map_rows::record_source(writer, context, row);
        retain(writer, context, row, assessment)
    })?;
    Err(unknown(context))
}

fn unknown(context: &SourceContext<'_>) -> CrawlError {
    CrawlError::PerformanceDateUnknown {
        published: context.page.meet.date.chars().take(64).collect(),
        as_of: context.performance_as_of,
    }
}

pub(super) fn admit(
    writer: &mut RowWriter<'_>,
    context: &SourceContext<'_>,
    row: &OwnedPerformance,
) -> CrawlResult<bool> {
    let assessment = assess_performance_date(context.performance_as_of, &context.page.meet.date);
    if matches!(assessment, PerformanceDateAssessment::Admitted) {
        return Ok(true);
    }
    retain(writer, context, row, assessment)?;
    match assessment {
        PerformanceDateAssessment::Unknown => Err(unknown(context)),
        _ => Ok(false),
    }
}

fn retain(
    writer: &mut RowWriter<'_>,
    context: &SourceContext<'_>,
    row: &OwnedPerformance,
    assessment: PerformanceDateAssessment,
) -> CrawlResult<()> {
    let disposition = match assessment {
        PerformanceDateAssessment::Future => "out_of_scope_future",
        _ => "retained_unresolved",
    };
    let key = format!(
        "observation/{}/{}/{}",
        row.meet_id, row.result_id, context.capture.content_digest
    );
    let retained = writer
        .accumulated
        .retained
        .get_mut(&key)
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| CrawlError::Invariant {
            detail: "owned performance date admission requires retained source evidence".into(),
        })?;
    retained.insert("disposition".into(), serde_json::json!(disposition));
    retained.insert("date".into(), serde_json::json!(context.page.meet.date));
    retained.insert(
        "performance_as_of".into(),
        serde_json::json!(context.performance_as_of),
    );
    retained.insert("provider".into(), serde_json::json!(row.provider));
    Ok(())
}
