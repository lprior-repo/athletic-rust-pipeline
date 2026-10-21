use super::*;
use census_domain::model::{AdjustmentKind, ExactSeconds, Mark};
use census_domain::UsJurisdiction;

mod provenance;

pub(super) fn observe(
    dataset: &ExportDataset,
    public: &PublicAthleteId,
    school: &SchoolId,
) -> TestResult<(CanonicalPerformance, SharedSelection)> {
    let [performance] = dataset.performances.as_slice() else {
        return Err("one genuine Tessa physical performance required".into());
    };
    physical(performance)?;
    identity(dataset, performance, public, school)?;
    let view = dataset.verified_participation()?;
    let assessed = view
        .assessment(&performance.id)
        .ok_or("genuine Tessa participation assessment")?;
    check!(eq; assessed.outcome(), crate::export::ParticipationOutcome::Eligible);
    check!(assessed.reasons().is_empty());
    check!(assessed.diagnostics().is_empty());
    check!(!assessed.evidence_refs().is_empty());
    let selected = crate::bests::build_with_exclusions(
        dataset,
        &crate::bests::Options {
            scope: crate::report::Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: vec![UsJurisdiction::Indiana],
        },
    )?;
    let [best] = selected.selections.as_slice() else {
        return Err("one genuine eligible adjusted Best required".into());
    };
    adjusted(best, performance)?;
    check!(eq; best.inventory.eligible_group_count(),1);
    check!(eq; best.inventory.tied_group_count(),1);
    check!(eq; best.inventory.tied_representative_ids(),std::slice::from_ref(&performance.id));
    check!(eq; best.athlete.grad_year,2027);
    Ok((performance.clone(), best.clone()))
}

fn physical(row: &CanonicalPerformance) -> TestResult {
    check!(eq; row.date,"2026-03-11");
    check!(eq; row.mark,Mark::TimeSeconds(ExactSeconds::parse("683.56")?));
    check!(row.place.is_none());
    let adjustment = row
        .adjustment
        .as_ref()
        .ok_or("genuine physical conversion")?;
    check!(eq; adjustment.kind,AdjustmentKind::TrackSizeConversion);
    check!(eq; adjustment.original_mark,Some(Mark::TimeSeconds(ExactSeconds::parse("689.42")?)));
    check!(eq; adjustment.adjusted_mark,row.mark);
    check!(adjustment
        .note
        .as_ref()
        .is_some_and(|note| !note.is_empty()));
    check!(eq; adjustment.source,None);
    provenance::verify(row)?;
    Ok(())
}

fn adjusted(best: &SharedSelection, row: &CanonicalPerformance) -> TestResult {
    let adjustment = row
        .adjustment
        .as_ref()
        .ok_or("original physical adjustment")?;
    check!(eq; best.source.performance_id,row.id);
    check!(eq; best.source.source_key,row.source_key);
    check!(eq; best.source.source_athlete,"8779607");
    check!(eq; best.source.result_url,captures::tfrrs()?.requested_url());
    check!(eq; best.meet.date,"2026-03-11");
    check!(eq; best.result.mark,row.mark);
    check!(eq; best.result.original_mark,adjustment.original_mark);
    check!(eq; best.result.adjustment_kind,Some(adjustment.kind));
    check!(eq; best.result.adjustment_basis,Some(adjustment.basis));
    check!(eq; best.result.adjustment_note,adjustment.note);
    check!(eq; best.result.adjustment_source,adjustment.source);
    check!(best.result.place.is_none());
    Ok(())
}

fn identity(
    dataset: &ExportDataset,
    row: &CanonicalPerformance,
    public: &PublicAthleteId,
    school: &SchoolId,
) -> TestResult {
    let identities = dataset.verified_public_identities()?;
    check!(eq; identities.public_id(row.athlete.as_str()),Some(public));
    let group = identities
        .group(row.athlete.as_str())
        .ok_or("genuine public identity group")?;
    check!(eq; group.canonical_candidate().as_str(),row.athlete.as_str());
    check!(eq; group.public_id(),Some(public));
    let affiliation = dataset.verified_current_affiliations()?;
    check!(eq; affiliation.as_of().as_str(),"2026-10-21");
    let resolution = affiliation.resolution(&row.athlete);
    check!(eq; resolution.school(),Some(school),"current roster resolution: {resolution:?}");
    check!(eq; resolution.status(),"resolved_at_selected_cutoff");
    let current = crate::export::current_school::SelectedSchools::of(dataset)?;
    let fields = current.fields(row.athlete.as_str())?;
    check!(eq; fields.current_school_id,school.as_str());
    check!(eq; fields.current_school,"Roncalli");
    check!(eq; fields.current_state,"IN");
    check!(eq; fields.current_school_as_of,"2026-10-21");
    Ok(())
}
