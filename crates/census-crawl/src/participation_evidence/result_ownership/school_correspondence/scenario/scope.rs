use super::*;
use census_domain::model::{Gender, ProfileDate, SchoolYear, Sport};
use census_store::StoreError;
fn changed_scope(mut scope: ResultSchoolScopeV1, variant: u8) -> TestResult<ResultSchoolScopeV1> {
    match variant {
        0 => scope.day = ProfileDate::parse("2026-05-16")?,
        1 => scope.sport = Sport::IndoorTrack,
        2 => scope.category = Gender::Girls,
        3 => scope.year = SchoolYear::new(2026).ok_or("alternate school year")?,
        _ => return Err("unknown scope case".into()),
    }
    Ok(scope)
}

#[test]
fn public_source_tuple_fields_enforce_original_scope_ownership() -> TestResult {
    let (_directory, store, input) =
        setup::install().map_err(|error| format!("install original sources: {error}"))?;
    attachment::apply(&store, &input)
        .map_err(|error| format!("review source attachments: {error}"))?;
    setup::enroll(&store).map_err(|error| format!("enroll correspondence review: {error}"))?;
    let proposal = review::proposal(&store, &input)
        .map_err(|error| format!("build original-school proposal: {error}"))?;
    check!(eq; proposal.claim().target.school, input.school);
    check!(eq; proposal.claim().target.day, ProfileDate::parse("2026-05-15")?);
    check!(eq; proposal.claim().target.sport, Sport::OutdoorTrack);
    check!(eq; proposal.claim().target.category, Gender::Boys);
    check!(eq; proposal.claim().target.year.get(), 2025);
    let before = setup::state(&store)?;
    for variant in 0..4 {
        let altered = changed_scope(scope()?, variant)?;
        let snapshot = store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let current =
            snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
        let provider = read_recorded_provider_school(
            &snapshot,
            &input.school_packet.reference()?,
            &mut grant,
        )?
        .ok_or("recorded provider school")?;
        let discovered = discover_current_html_results(&current, &mut grant)?;
        let result = discovered.with_group(0, &mut grant, |linked, grant| {
            build_claim(&current, &linked, 0, &provider, altered, grant)
        });
        match result {
            Err(crate::CrawlError::Store(StoreError::Refused { .. })) => {}
            Err(error) => {
                return Err(format!("wrong scope refusal variant {variant}: {error}").into());
            }
            Ok(_) => {
                return Err(
                    format!("altered scope variant {variant} built source proposal").into(),
                );
            }
        }
        check!(eq; setup::state(&store)?, before);
    }
    Ok(())
}
