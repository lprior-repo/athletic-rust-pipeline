use super::*;
use crate::participation_evidence::recorded::{
    with_frozen_attached_participation, DocumentarySeasonApplicabilityV1,
};
use crate::participation_evidence::ParticipationEvidenceRefV1;
use census_domain::model::StoreIdentity;

pub(super) struct Proof {
    school: Vec<u8>,
    school_reference: AttachmentObjectReferenceV1,
    calendar: Vec<u8>,
    calendar_reference: AttachmentObjectReferenceV1,
    store: StoreIdentity,
    sequence: u64,
}

pub(super) fn capture(store: &Store) -> TestResult<Proof> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let schools =
        snapshot.read_current_result_school_correspondences_v1(&attachments, &mut grant)?;
    let calendars =
        snapshot.read_current_result_calendar_correspondences_v1(&attachments, &mut grant)?;
    let identity = snapshot.freeze_public_identity_v2()?;
    let school = schools.freeze_closure(&identity, &mut grant)?;
    let calendar = calendars.freeze_closure(&identity, &mut grant)?;
    Ok(Proof {
        school: school.bytes().to_vec(),
        school_reference: school.reference().clone(),
        calendar: calendar.bytes().to_vec(),
        calendar_reference: calendar.reference().clone(),
        store: calendars.store_identity().clone(),
        sequence: calendars.snapshot_sequence(),
    })
}

pub(super) fn observe(
    proof: &Proof,
    target: &SchoolId,
    reference: &ParticipationEvidenceRefV1,
    expected: Option<&ResultCalendarDigestV1>,
) -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let schools = census_store::replay_result_school_source_proof_v1(
        &proof.school,
        &proof.school_reference,
        &proof.store,
        proof.sequence,
        &mut grant,
    )?;
    let calendars = census_store::replay_result_calendar_source_proof_v1(
        &proof.calendar,
        &proof.calendar_reference,
        &proof.store,
        proof.sequence,
        &mut grant,
    )?;
    let attachments = calendars.attachments(&mut grant)?;
    let plan = discover_frozen_html_results(attachments, &mut grant)?;
    check!(eq; plan.group_count(), 1);
    check!(eq; plan.diagnostic_count(), 0);
    let checked = plan.with_group(0, &mut grant, |linked, grant| {
        let school =
            school::read::checked_frozen(&schools, &linked, 0, grant)?.ok_or_else(|| {
                crate::participation_evidence::invalid("exact frozen Roncalli school absent")
            })?;
        with_frozen_attached_participation(attachments, grant, |sources, grant| {
            sources.with_row_facts(&linked, 0, grant, |facts, grant| {
                let checked =
                    calendar::read::checked_frozen(&calendars, &linked, 0, &facts, grant)?;
                let selected = checked.as_ref().map(|value| {
                    (
                        value.documentary_reference().clone(),
                        value
                            .record()
                            .command
                            .source
                            .calendar
                            .interpretation
                            .clone(),
                    )
                });
                let physical = physical::read(&linked, &school);
                let school = school.school().clone();
                let documentary = facts
                    .sources()
                    .iter()
                    .map(|fact| {
                        (
                            fact.season(),
                            fact.source().competition().to_owned(),
                            fact.source().venue().to_owned(),
                        )
                    })
                    .collect::<Vec<_>>();
                Ok((school, selected, documentary, physical))
            })
        })
    })?;
    check!(eq; &checked.0, target);
    checked.3?;
    let [(season, competition, venue)] = checked.2.as_slice() else {
        return Err("single exact-day Roncalli fact".into());
    };
    check!(eq; *season, DocumentarySeasonApplicabilityV1::ReviewedCalendarRequired);
    check!(eq; competition, "at HSR Indoor Meet");
    check!(eq; venue, "UIndy");
    match expected {
        Some(expected) => check!(eq; checked.1, Some((reference.clone(), expected.clone()))),
        None => check!(checked.1.is_none()),
    }
    grant.ensure_available()?;
    Ok(())
}
