use super::*;
use crate::participation_evidence::recorded::{
    with_current_attached_participation, with_frozen_attached_participation,
    AttachedParticipationV1, DocumentaryCohortAgreementV1, DocumentarySeasonApplicabilityV1,
};
use census_domain::model::{AdjustmentKind, CanonicalSchool};

pub(super) fn current(
    store: &Store,
    athlete: &CanonicalAthlete,
    school: &CanonicalSchool,
) -> TestResult<Vec<u8>> {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted native Tessa target")?;
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = snapshot.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted, &mut grant,
    )?;
    let observed = with_current_attached_participation(&owner, &mut grant, |sources, grant| {
        let plan = discover_current_html_results(&owner, grant)?;
        if plan.group_count() != 1 || plan.diagnostic_count() != 0 {
            return Err(crate::participation_evidence::invalid(
                "exact Tessa result discovery changed",
            )
            .into());
        }
        plan.with_group(0, grant, |linked, grant| {
            Ok(view(sources, &linked, athlete, school, grant))
        })
    })??;
    grant.ensure_available()?;
    Ok(observed)
}

pub(super) fn frozen(
    accepted: &census_store::AcceptedPublicIdentitySnapshot,
    bundle: &FrozenEvidenceSubjectAttachmentBundleV1,
    athlete: &CanonicalAthlete,
    school: &CanonicalSchool,
) -> TestResult<Vec<u8>> {
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = census_store::replay_frozen_evidence_subject_attachment_references_v1(
        bundle, accepted, &mut grant,
    )?;
    let observed = with_frozen_attached_participation(&owner, &mut grant, |sources, grant| {
        let plan = discover_frozen_html_results(&owner, grant)?;
        if plan.group_count() != 1 || plan.diagnostic_count() != 0 {
            return Err(crate::participation_evidence::invalid(
                "exact frozen Tessa discovery changed",
            )
            .into());
        }
        plan.with_group(0, grant, |linked, grant| {
            Ok(view(sources, &linked, athlete, school, grant))
        })
    })??;
    grant.ensure_available()?;
    Ok(observed)
}

fn view<'s>(
    sources: &AttachedParticipationV1<'_, 's>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, 's>,
    athlete: &CanonicalAthlete,
    school: &CanonicalSchool,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<Vec<u8>> {
    check!(eq; linked.row_count(), 1);
    check!(linked.row(1).is_none());
    let CheckedHtmlResultDocumentV1::Tfrrs(checked) = linked.checked_document() else {
        return Err("exact Tessa original TFRRS document".into());
    };
    let source = checked.row(0).ok_or("original checked Tessa row")?;
    check!(eq; source.locator(), source::locator());
    check!(eq; source.original_row_digest(), "d33bf3fb6858f07f8e41576232afd0bf89c848e420fbe67bef779f0be6f1df8d");
    check!(eq; source.athlete_id(), "8779607");
    check!(eq; source.team_path(), "/teams/tf/Roncalli_f.html");
    check!(eq; source.team_name(), school.name);
    check!(eq; source.athlete_name(), athlete.canonical_name);
    check!(eq; source.grade().get(), 11);
    check!(eq; source.published_date(), "2026-03-11");
    check!(eq; source.original_mark(), Some("11:29.42"));
    check!(eq; source.published_mark(), "11:23.56");
    check!(source.original_row()?.contains("/tie_break/5489/36792/44"));
    let row = linked.row(0).ok_or("identity-linked native Tessa row")?;
    check!(eq; row.attachment().target().canonical_candidate, athlete.id.cast());
    check!(eq; row.gaps(), &[crate::participation_evidence::recorded::AttachedResultGapV1::SchoolBindingUnresolved]);
    let projected = project(linked, athlete, school)?;
    let facts = sources.with_row_facts(linked, 0, grant, |facts, _| Ok(documentary(facts)))??;
    Ok(serde_json::to_vec(&(
        linked.recorded_root(),
        row.attachment().target(),
        source.locator(),
        source.reference(),
        source.context_reference(),
        row.attachment().subject(),
        projected,
        facts,
    ))?)
}

fn documentary(
    facts: crate::participation_evidence::recorded::CheckedDocumentaryParticipationFactsV1<'_>,
) -> TestResult<Vec<u8>> {
    let [fact] = facts.sources() else {
        return Err("one attached exact-event-day Roncalli authority".into());
    };
    check!(facts.windows().is_empty());
    check!(eq; fact.season(), DocumentarySeasonApplicabilityV1::ReviewedCalendarRequired);
    check!(eq; fact.cohort_agreement(), DocumentaryCohortAgreementV1::Agreed);
    let source = fact.source();
    check!(eq; source.subject().document_namespace(), "roncalli_girls_track_season_review_v1");
    check!(eq; source.subject().name(), "Tessa Mize");
    check!(eq; source.subject().school(), "Roncalli");
    check!(eq; source.day().as_str(), "2026-03-11");
    check!(eq; source.cohort(), GradYear::CO2027);
    check!(eq; source.competition(), "at HSR Indoor Meet");
    check!(eq; source.venue(), "UIndy");
    let candidate = source
        .roncalli_event()
        .ok_or("retained checked school event facts")?;
    check!(eq; candidate.status().grade, 11);
    check!(eq; candidate.event().original_mark, "11:29.42");
    check!(eq; candidate.event().place, 2);
    Ok(serde_json::to_vec(&(
        source.reference(),
        source.subject().reference(),
        source.day(),
        source.school_year(),
        source.cohort(),
        source.competition(),
        source.venue(),
        source.status_proof().capture(),
        source.status_proof().interpretation(),
        source.season_proof().capture(),
        source.season_proof().locator(),
        fact.attachment_record_digest(),
    ))?)
}

fn project(
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    athlete: &CanonicalAthlete,
    school: &CanonicalSchool,
) -> TestResult<Vec<u8>> {
    let source = linked.row(0).ok_or("identity-linked Tessa result")?;
    let CheckedHtmlResultDocumentV1::Tfrrs(document) = linked.checked_document() else {
        return Err("genuine Tessa TFRRS document".into());
    };
    let row = document.row(0).ok_or("genuine selected Tessa row")?;
    let subject = row.subject()?.reference();
    let unresolved =
        objects::reference(b"controlled-unadmitted-school-reference-no-eligibility-claim")?;
    let target = ProposedHtmlResultTargetV1 {
        locator: row.locator(),
        subject_reference: &subject,
        proposed_athlete: &athlete.id,
        proposed_school: &school.id,
        proposed_attachment_reference: source.attachment().record().record_digest.as_str(),
        proposed_school_binding_reference: unresolved.digest.as_str(),
    };
    let (meet, projected) = project_checked_html_result_row(linked.checked_document(), 0, &target)?;
    check!(eq; meet.date, "2026-03-11");
    let performance = projected.proposed_performance();
    check!(eq; performance.mark, Mark::TimeSeconds(ExactSeconds::parse("683.56")?));
    check!(performance.place.is_none());
    let adjustment = performance
        .adjustment
        .as_ref()
        .ok_or("genuine provider track conversion")?;
    check!(eq; adjustment.kind, AdjustmentKind::TrackSizeConversion);
    check!(eq; adjustment.original_mark, Some(Mark::TimeSeconds(ExactSeconds::parse("689.42")?)));
    check!(eq; adjustment.note.as_deref(), row.conversion_note());
    check!(eq; performance.source_athlete.as_ref().ok_or("native performance person")?.id, "8779607");
    Ok(serde_json::to_vec(&(meet, projected))?)
}
