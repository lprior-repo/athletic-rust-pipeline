use super::types::{
    DiscoveredIdentityLinkedHtmlResultDocumentV1, DiscoveredIdentityLinkedHtmlResultRowV1,
    DiscoveredResultAttachmentV1,
};
use crate::milesplit::parse_published_time;
use crate::participation_evidence::provider_target::{
    CheckedLinkedCompetitionCategoryV1, CheckedLinkedEventLabelV1, SourceTargetPacketV1,
};
use crate::participation_evidence::{invalid, result_ownership::CheckedRetainedResultRowV1};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{
    AttachmentCorroborationBasisV1, AttachmentFactDispositionV1, AttachmentReplayGrantV1,
};
use census_domain::model::{EventKind, EventSpecification, GradYear, Mark, SourceIdentity};
use census_domain::public_identity_history::{BindingResolution, PublicResolution};
use census_store::{
    ReadAttachmentContextObjectV1, RetainedCaptureLeafKindV1, RetainedCaptureRootSchemaV1,
    RetainedCaptureSlotV1,
};

pub(super) fn read(
    document: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<CheckedLinkedEventLabelV1>> {
    super::super::read::with_grant(grant, |grant| read_bound(document, index, grant))
}

fn read_bound(
    document: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<CheckedLinkedEventLabelV1>> {
    let row = document
        .row(index)
        .ok_or_else(|| invalid("checked event label result row absent"))?;
    let attachment = row.attachment();
    attachment.require_original_grant(grant)?;
    let objects = attachment
        .record()
        .command
        .reviewed_context
        .objects
        .as_slice();
    let mut matched = None;
    for reference in objects {
        grant.charge_steps(1).map_err(super::domain)?;
        let Some(ReadAttachmentContextObjectV1::Retained(leaf)) =
            attachment.read_context_object(reference, grant)?
        else {
            continue;
        };
        if leaf.root.schema != RetainedCaptureRootSchemaV1::ProviderTargetPacketV1
            || leaf.slot != RetainedCaptureSlotV1::ProviderLinkedResults
            || leaf.kind != RetainedCaptureLeafKindV1::CaptureJson
        {
            continue;
        }
        let Some((packet, held)) = super::super::read::retained_from::<SourceTargetPacketV1>(
            attachment.source(),
            leaf.root.schema,
            &leaf.root.key,
            grant,
        )?
        else {
            return Err(invalid("approved linked-event provider packet absent").into());
        };
        if held.root() != &leaf.root || packet.reference()?.as_str() != leaf.root.key {
            return Err(invalid("approved linked-event provider root differs").into());
        }
        retain_packet_label(document, &row, &packet, grant, &mut matched)?;
    }
    Ok(matched)
}

fn retain_packet_label(
    document: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    row: &DiscoveredIdentityLinkedHtmlResultRowV1<'_, '_>,
    packet: &SourceTargetPacketV1,
    grant: &mut AttachmentReplayGrantV1,
    matched: &mut Option<CheckedLinkedEventLabelV1>,
) -> CrawlResult<()> {
    let attachment = row.attachment();
    let checked = packet.read()?;
    let profile = checked.profile();
    if !owns_person(attachment, profile.person(), grant)? {
        return Ok(());
    }
    let raw = row.checked_row();
    if raw.parsed_row().name != profile.name() || raw.parsed_row().school != profile.school_name() {
        return Err(invalid("linked-event source person differs from approved result").into());
    }
    if raw.parsed_row().grade.is_some_and(|grade| {
        GradYear::of(grade, document.checked_document().parsed_page().school_year)
            != Some(profile.cohort())
    }) {
        return Err(invalid("linked-event source cohort contradicts result grade").into());
    }
    for linked in checked.categories().rows() {
        grant.charge_steps(1).map_err(super::domain)?;
        if !same_result(document, raw, linked)? {
            continue;
        }
        let label = packet.read_event_label(linked.proof())?;
        if !corroborates(attachment, raw, &label, grant)? {
            continue;
        }
        require_matching_event(&raw.event().kind, &raw.event().label, label.label())?;
        if matched.replace(label).is_some() {
            return Err(invalid("approved linked event source is ambiguous").into());
        }
    }
    Ok(())
}

fn owns_person(
    attachment: &DiscoveredResultAttachmentV1<'_, '_>,
    person: &SourceIdentity,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<bool> {
    let accepted = attachment.accepted_public_identities();
    let target = &attachment.target().public_id;
    let members = accepted.component_bindings(target)?;
    for id in members {
        grant.charge_steps(1).map_err(super::domain)?;
        let Some(claim) = accepted.binding_claim(&id) else {
            continue;
        };
        if claim.source.namespace == person.namespace && claim.source.identifier == person.id {
            return Ok(matches!(
                accepted.resolve_binding(&id)?,
                BindingResolution::Resolved {
                    identity: PublicResolution::Canonical(resolved),
                    superseded_by: None,
                    ..
                } if resolved.public_id == *target
            ));
        }
    }
    Ok(false)
}

fn same_result(
    document: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    raw: &CheckedRetainedResultRowV1<'_>,
    linked: &CheckedLinkedCompetitionCategoryV1,
) -> CrawlResult<bool> {
    let checked = document.checked_document();
    if linked.meet_id() != checked.meet_id()
        || linked.result_document_id() != checked.result_document_id()
        || linked.meet_day().as_str() != checked.published_start()
        || linked.category() != Some(raw.event().gender)
    {
        return Ok(false);
    }
    let Some((time, timing)) = parse_published_time(linked.mark()) else {
        return Ok(false);
    };
    Ok(raw.parsed_row().mark == Mark::TimeSeconds(time)
        && timing.is_none_or(|value| raw.parsed_row().timing == Some(value)))
}

fn require_matching_event(
    original: &EventKind,
    original_label: &str,
    linked_label: &str,
) -> CrawlResult<()> {
    let kind = EventKind::from_source_label(linked_label);
    if &kind != original
        || EventSpecification::from_published_label(linked_label, &kind)?
            != EventSpecification::from_published_label(original_label, &kind)?
    {
        return Err(invalid("checked linked event contradicts original event").into());
    }
    Ok(())
}

fn corroborates(
    attachment: &DiscoveredResultAttachmentV1<'_, '_>,
    raw: &CheckedRetainedResultRowV1<'_>,
    label: &CheckedLinkedEventLabelV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<bool> {
    let context = &attachment.record().command.reviewed_context;
    for pair in context.corroboration.as_slice() {
        grant.charge_steps(1).map_err(super::domain)?;
        if pair.basis != AttachmentCorroborationBasisV1::ExplicitOriginalCrossReference
            || pair.subject_fact.as_str() != raw.original_row_digest()
            || pair.accepted_target_fact != label.row_reference().digest
        {
            continue;
        }
        for fact in context.facts.as_slice() {
            grant.charge_steps(1).map_err(super::domain)?;
            if fact.fact == pair.accepted_target_fact
                && fact.disposition == AttachmentFactDispositionV1::SupportsAttachment
            {
                for subject in context.accepted_target_subjects.as_slice() {
                    grant.charge_steps(1).map_err(super::domain)?;
                    if subject == &fact.subject {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests;
