use super::{domain, invalid, reserve, Member};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
};
use census_domain::participation_candidates::MAX_PARTICIPATION_FACTS;
use census_store::{
    ActiveAttachmentSubjectDiscoveryV1, DiscoveredAttachmentCaptureV1, RetainedCaptureRootV1,
};
use std::cmp::Ordering;
use std::ops::Range;

pub(super) fn insert(
    metadata: &ActiveAttachmentSubjectDiscoveryV1<'_>,
    members: &mut Vec<Member>,
    member: Member,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    let mut lower = 0usize;
    let mut upper = members.len();
    while lower < upper {
        let half = upper
            .checked_sub(lower)
            .ok_or(census_store::StoreError::CounterOverflow)?
            / 2;
        let middle = lower
            .checked_add(half)
            .ok_or(census_store::StoreError::CounterOverflow)?;
        let prior = members
            .get(middle)
            .ok_or_else(|| invalid("discovery member absent"))?;
        let order = compare(metadata, prior.index, member.index, grant)?;
        grant.charge_steps(1).map_err(domain)?;
        let order = order.then_with(|| prior.span.cmp(&member.span));
        if order == Ordering::Less {
            lower = middle
                .checked_add(1)
                .ok_or(census_store::StoreError::CounterOverflow)?;
        } else {
            upper = middle;
        }
    }
    let shifted = members
        .len()
        .checked_sub(lower)
        .ok_or(census_store::StoreError::CounterOverflow)?;
    grant.charge_steps(shifted).map_err(domain)?;
    members.insert(lower, member);
    Ok(())
}

pub(super) fn ranges(
    metadata: &ActiveAttachmentSubjectDiscoveryV1<'_>,
    members: &[Member],
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Vec<Range<usize>>> {
    let mut groups = reserve(members.len(), grant)?;
    let mut start = 0usize;
    let mut previous: Option<usize> = None;
    for (index, member) in members.iter().enumerate() {
        grant.charge_steps(1).map_err(domain)?;
        if let Some(prior) = previous {
            if compare(metadata, prior, member.index, grant)? != Ordering::Equal {
                push(&mut groups, start..index)?;
                start = index;
            }
        }
        previous = Some(member.index);
    }
    if previous.is_some() {
        push(&mut groups, start..members.len())?;
    }
    Ok(groups)
}

fn push(groups: &mut Vec<Range<usize>>, range: Range<usize>) -> CrawlResult<()> {
    let count = range
        .end
        .checked_sub(range.start)
        .ok_or(census_store::StoreError::CounterOverflow)?;
    if count > MAX_PARTICIPATION_FACTS {
        return Err(super::super::super::read::ownership(
            crate::participation_evidence::result_ownership::RetainedResultOwnershipError::Capacity {
                needed: count, available: MAX_PARTICIPATION_FACTS,
            },
        ));
    }
    groups.push(range);
    Ok(())
}

fn compare(
    metadata: &ActiveAttachmentSubjectDiscoveryV1<'_>,
    left: usize,
    right: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Ordering> {
    grant.charge_steps(1).map_err(domain)?;
    let (left_root, left_context) = key(metadata, left)?;
    let (right_root, right_context) = key(metadata, right)?;
    let root = (left_root.schema.phase(), &left_root.key)
        .cmp(&(right_root.schema.phase(), &right_root.key));
    if root == Ordering::Equal && left_root.payload != right_root.payload {
        return Err(invalid("discovery capture root descriptors conflict").into());
    }
    Ok(root
        .then_with(|| left_root.payload.cmp(&right_root.payload))
        .then_with(|| left_context.cmp(right_context)))
}

pub(super) fn key<'a>(
    metadata: &'a ActiveAttachmentSubjectDiscoveryV1<'_>,
    index: usize,
) -> CrawlResult<(&'a RetainedCaptureRootV1, &'a AttachmentObjectReferenceV1)> {
    let source = metadata
        .subjects()
        .get(index)
        .ok_or_else(|| invalid("discovery subject absent"))?;
    let DiscoveredAttachmentCaptureV1::Retained(leaf) = source.capture() else {
        return Err(invalid("supported discovery capture absent").into());
    };
    Ok((&leaf.root, source.context()))
}
