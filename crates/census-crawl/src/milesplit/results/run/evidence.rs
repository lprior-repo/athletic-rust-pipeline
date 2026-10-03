use super::super::Accumulator;
use super::{capture, RawPage, ResultSetRef};
use crate::net::FetchOutcome;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{Evidence, SourceRef};

pub(super) fn bind(
    projected: &mut Accumulator,
    reference: &ResultSetRef,
    page: &RawPage,
    owned: &FetchOutcome,
    metadata: &FetchOutcome,
) -> CrawlResult<()> {
    let owned = capture::provenance(owned);
    let raw = capture::metadata_provenance(reference, metadata)?;
    let evidence = metadata_evidence(reference, page, metadata, &owned, &raw)?;
    projected
        .meets
        .values_mut()
        .map(|row| &mut row.evidence)
        .chain(projected.events.values_mut().map(|row| &mut row.evidence))
        .chain(projected.teams.values_mut().map(|row| &mut row.evidence))
        .chain(projected.athletes.values_mut().map(|row| &mut row.evidence))
        .chain(
            projected
                .performances
                .values_mut()
                .map(|row| &mut row.evidence),
        )
        .for_each(|held| bind_evidence(held, &evidence));
    projected.retained.values_mut().try_for_each(|row| {
        let fields = row.as_object_mut().ok_or_else(|| CrawlError::Invariant {
            detail: "retained result projection is not an object".into(),
        })?;
        fields.insert("owned_capture".into(), owned.clone());
        fields.insert("raw_metadata_capture".into(), raw.clone());
        Ok(())
    })
}

fn metadata_evidence(
    reference: &ResultSetRef,
    page: &RawPage,
    metadata: &FetchOutcome,
    owned: &serde_json::Value,
    raw: &serde_json::Value,
) -> CrawlResult<Evidence> {
    let mut evidence = Evidence::parsed(
        SourceRef::new(reference.site.source_id(), Some(reference.url.clone())),
        &metadata.fetched_at,
    );
    evidence.note = Some(
        serde_json::to_string(&serde_json::json!({
            "role": "raw result file meet/season metadata only",
            "meet_id": reference.meet_id, "result_set_id": reference.rsid,
            "meet_name": page.meet.name, "meet_date": page.meet.date,
            "region": page.region, "sport": page.sport, "school_year": page.school_year,
            "owned_capture": owned, "raw_metadata_capture": raw,
            "projection_revision": super::super::RESULT_SET_PHASE,
            "parser_revision": super::receipt::PARSER_REVISION,
        }))
        .map_err(|source| CrawlError::Encode {
            table: "result projection metadata evidence".into(),
            source,
        })?,
    );
    Ok(evidence)
}

fn bind_evidence(held: &mut Vec<Evidence>, metadata: &Evidence) {
    let mut replaced = false;
    held.iter_mut()
        .filter(|evidence| evidence.source == metadata.source)
        .for_each(|evidence| {
            evidence.clone_from(metadata);
            replaced = true;
        });
    if !replaced {
        held.push(metadata.clone());
    }
}
