use super::types::{FinalUrl, IssuanceWire, ProposalWire, RecordWire, RosterAuthorityObjectRefV1};
use super::{
    charge, size, Result, RosterAuthorityError, RosterAuthorityLimitsV1, RosterAuthorityScopeV1,
    RosterCaptureClaimV1, RosterExistingBindingClaimV1, MAX_BODY_BYTES, MAX_OBJECT_BYTES,
};
use crate::model::{Gender, SourceNamespace, Sport};
use chrono::{DateTime, Utc};

pub(super) fn issuance(raw: &IssuanceWire, limits: RosterAuthorityLimitsV1) -> Result<()> {
    if raw.version != 1 {
        return Err(RosterAuthorityError::UnsupportedVersion);
    }
    scope(&raw.scope, &raw.binding)?;
    capture(&raw.capture, &raw.binding)?;
    if raw.parser.interpretation.as_str() != "milesplit-scoped-current-roster/v1" {
        return Err(RosterAuthorityError::Invalid("parser interpretation"));
    }
    check_references(&references(raw), &[], limits)
}
pub(super) fn proposal(raw: &ProposalWire, limits: RosterAuthorityLimitsV1) -> Result<()> {
    if raw.version != 1 {
        return Err(RosterAuthorityError::UnsupportedVersion);
    }
    scope(&raw.scope, &raw.binding)?;
    capture(&raw.capture, &raw.binding)?;
    if raw.parser.interpretation.as_str() != "milesplit-scoped-current-roster/v1" {
        return Err(RosterAuthorityError::Invalid("parser interpretation"));
    }
    let refs = [
        &raw.capture.body,
        &raw.binding.provider_packet,
        &raw.parser.artifact,
        &raw.parser.schema,
        &raw.parser.configuration,
        &raw.parser.dependencies,
        &raw.capture.manifest,
        &raw.capture.interpretation,
        &raw.capture.retained_source_admission,
        &raw.capture.verification_grant,
        &raw.binding.school_projection,
        &raw.binding.program_projection,
        &raw.binding.physical_projection_inventory,
    ];
    check_references(&refs, &[], limits)
}

fn scope(scope: &RosterAuthorityScopeV1, binding: &RosterExistingBindingClaimV1) -> Result<()> {
    if scope.sport == Sport::Unknown || scope.category == Gender::Unknown {
        return Err(RosterAuthorityError::Invalid("unknown sport or category"));
    }
    if scope.owner.namespace() != &SourceNamespace::MilesplitSchool
        || binding.school_source != scope.owner
        || binding.program_source.namespace() != &SourceNamespace::MilesplitTeam
        || binding.program_source.provider_id() != scope.owner.provider_id()
        || binding.sport != scope.sport
        || binding.category != scope.category
    {
        return Err(RosterAuthorityError::BindingScopeMismatch);
    }
    provider_key(scope.owner.provider_id())?;
    identifier(binding.school.as_str())?;
    identifier(binding.program.as_str())
}

fn provider_key(value: &str) -> Result<()> {
    if value.len() > 20
        || value.starts_with('0')
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || !value.parse::<u64>().is_ok_and(|id| id > 0)
    {
        return Err(RosterAuthorityError::Invalid("provider key"));
    }
    Ok(())
}

fn identifier(value: &str) -> Result<()> {
    charge("identifier bytes", size(value.len())?, 512)?;
    if value.is_empty()
        || value
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control())
    {
        return Err(RosterAuthorityError::Invalid("identifier"));
    }
    Ok(())
}

fn capture(capture: &RosterCaptureClaimV1, binding: &RosterExistingBindingClaimV1) -> Result<()> {
    charge(
        "acquisition timestamp bytes",
        size(capture.acquired_at.as_str().len())?,
        64,
    )?;
    let instant = DateTime::parse_from_rfc3339(capture.acquired_at.as_str())
        .map_err(|_| RosterAuthorityError::Invalid("acquisition instant"))?;
    if instant.with_timezone(&Utc).date_naive().to_string() != capture.observed_day.as_str() {
        return Err(RosterAuthorityError::AcquisitionDayMismatch);
    }
    source_url(capture.requested_url.as_str())?;
    if let FinalUrl::Observed(url) = &capture.final_url {
        source_url(url.as_str())?;
    }
    charge(
        "roster body bytes",
        capture.body.bytes.get(),
        MAX_BODY_BYTES,
    )?;
    charge(
        "provider packet bytes",
        binding.provider_packet.bytes.get(),
        MAX_BODY_BYTES,
    )?;
    Ok(())
}

fn source_url(value: &str) -> Result<()> {
    let url = url::Url::parse(value).map_err(|_| RosterAuthorityError::Invalid("source URL"))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || value.chars().any(char::is_whitespace)
    {
        return Err(RosterAuthorityError::Invalid("source URL"));
    }
    Ok(())
}

pub(super) fn record(raw: &RecordWire, limits: RosterAuthorityLimitsV1) -> Result<()> {
    if raw.version != 1 {
        return Err(RosterAuthorityError::UnsupportedVersion);
    }
    if raw.lifecycle.as_str() != "issuance_only_v1" || raw.negative_actions != 0 {
        return Err(RosterAuthorityError::UnsupportedLifecycle);
    }
    identifier(raw.operation.as_str())?;
    if raw.store != raw.proposal.binding.store
        || raw.snapshot_revision != raw.proposal.binding.snapshot_revision
    {
        return Err(RosterAuthorityError::BindingScopeMismatch);
    }
    issuance(&raw.proposal, limits)?;
    check_references(
        &references(&raw.proposal),
        &[
            &raw.parent_inventory,
            &raw.negative_inventory,
            &raw.approval_envelope,
            &raw.review_provenance,
            &raw.issuance_receipt,
        ],
        limits,
    )
}

fn references(raw: &IssuanceWire) -> [&RosterAuthorityObjectRefV1; 14] {
    [
        &raw.capture.body,
        &raw.binding.provider_packet,
        &raw.parser.artifact,
        &raw.parser.schema,
        &raw.parser.configuration,
        &raw.parser.dependencies,
        &raw.capture.manifest,
        &raw.capture.interpretation,
        &raw.capture.retained_source_admission,
        &raw.capture.verification_grant,
        &raw.binding.school_projection,
        &raw.binding.program_projection,
        &raw.binding.physical_projection_inventory,
        &raw.semantic_review_evidence,
    ]
}

fn check_references(
    primary: &[&RosterAuthorityObjectRefV1],
    extra: &[&RosterAuthorityObjectRefV1],
    limits: RosterAuthorityLimitsV1,
) -> Result<()> {
    charge("record proof references", size(extra.len())?, 5)?;
    let mut bytes = 0u64;
    for (index, reference) in primary.iter().chain(extra).enumerate() {
        let ceiling = if index < 2 {
            MAX_BODY_BYTES
        } else {
            MAX_OBJECT_BYTES
        };
        charge("referenced object bytes", reference.bytes.get(), ceiling)?;
        let mut duplicate = false;
        for previous in primary.iter().chain(extra).take(index) {
            if previous.digest == reference.digest {
                if previous.bytes != reference.bytes {
                    return Err(RosterAuthorityError::ConflictingObjectReference);
                }
                duplicate = true;
            }
        }
        if !duplicate {
            bytes = bytes
                .checked_add(reference.bytes.get())
                .ok_or(RosterAuthorityError::Overflow)?;
            charge("component bytes", bytes, limits.component_bytes)?;
        }
    }
    Ok(())
}
