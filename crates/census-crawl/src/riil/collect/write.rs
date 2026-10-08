use crate::directory::acquisition::publish;
use crate::net::FetchOutcome;
use crate::riil::map::{capture_note, SchoolExtract};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalSchool, SourceNamespace, SourceObservation, SourceSchoolObservation,
};
use census_store::Table;

pub(super) fn owner(
    ctx: &AdapterContext<'_>,
    extract: &SchoolExtract,
    capture: &FetchOutcome,
    locator: &str,
    report: &mut AdapterReport,
) -> CrawlResult<bool> {
    let written = publish(
        ctx,
        ("riil", locator),
        Table::Schools,
        std::slice::from_ref(&extract.school),
        report,
    )?;
    let observation = SourceSchoolObservation::of_school(
        &SourceNamespace::association_school(crate::riil::ASSOCIATION),
        &extract.school,
        &capture.fetched_at,
    )
    .map(|mut observation| {
        observation.source_row_key = capture_note(capture).to_string();
        SourceObservation::School(observation)
    });
    publish(
        ctx,
        ("riil", locator),
        Table::SourceObservations,
        observation.as_slice(),
        report,
    )?;
    Ok(written > 0)
}

pub(super) fn complete(
    ctx: &AdapterContext<'_>,
    school: &CanonicalSchool,
    capture: &FetchOutcome,
    coaches: usize,
) -> CrawlResult<()> {
    let key = format!("RI:{}:{}", school.id, capture.content_digest);
    let payload = serde_json::json!({"name": school.name, "coaches": coaches, "capture": capture_note(capture)});
    let digest =
        census_domain::model::serialized_digest(&("northern_projection_v3", &key, &payload))
            .map_err(|source| CrawlError::Canonical {
                table: "riil_schools".into(),
                source,
            })?;
    let locator =
        census_domain::model::serialized_digest(&key).map_err(|source| CrawlError::Canonical {
            table: "riil_schools".into(),
            source,
        })?;
    let mut batch = ctx.write_batch();
    batch.journal_done("riil_schools", &key, &payload)?;
    batch
        .commit_once(
            &format!("northern_projection_v3:riil/completion:{locator}"),
            &digest,
        )
        .map(|_| ())
}
