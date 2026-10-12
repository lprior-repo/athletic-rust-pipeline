use super::*;

pub(super) const PHASE: &str = "meet_fragment_diagnostics_v1";

pub(super) fn bounded_census(
    store: &Store,
    mut census: MeetCensus,
    identity: &str,
    ordinal: usize,
) -> Result<MeetCensus, HandlerError> {
    let digest = census_domain::model::serialized_digest(&census)
        .map_err(|error| jobs::invariant(&format!("digesting meet diagnostics: {error}")))?;
    store
        .journal_done(PHASE, identity, &census)
        .map_err(store_error)?;
    let base = identity.rsplit_once('/').map_or(identity, |(base, _)| base);
    let reference = format!("{PHASE}/{identity}@{digest}");
    for source in &mut census.sources {
        if !source.errors.is_empty() {
            source.errors = vec![format!(
                "{} errors; diagnostics={reference}",
                source.errors.len()
            )];
        }
        if !source.unfinished.is_empty() {
            source.unfinished = vec![format!(
                "{} unfinished items; diagnostics={reference}",
                source.unfinished.len()
            )];
        }
        source.notes = vec![format!(
            "diagnostics={PHASE}/{base}/0..={ordinal}; latest={digest}"
        )];
    }
    for frontier in &mut census.frontiers {
        if let Some(failure) = &mut frontier.failure {
            failure.detail = reference.clone();
        }
    }
    Ok(census)
}
