use super::super::source_binding::{BoundRosterSourceViewV1, SourceBindingObjectV1};
use super::*;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::CanonicalAthlete;
use census_store::clock::{Clock, SystemClock};
use census_store::Table;
use chrono::{DateTime, Utc};

pub(super) fn capture(
    dataset: &ExportDataset,
    snapshot: &StoreSnapshot<'_>,
    objects: &Path,
    profile: &QueryProfileV1,
) -> ReportResult<(FrozenAffiliationDescriptor, PinnedCurrentAffiliations)> {
    let mut grant = AttachmentReplayGrantV1::new();
    let result = (|| {
        let claims = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
        let captured_at = DateTime::parse_from_rfc3339(&SystemClock.today_iso8601())
            .map_err(|_| invalid("reviewed roster capture clock is invalid"))?
            .with_timezone(&Utc);
        let profile_object = encode(objects, profile, PROFILE_BYTES)?;
        let observations = physical_observations::capture::<CanonicalAthlete>(
            dataset,
            snapshot,
            Table::Athletes,
            objects,
            observation_budget(),
        )?;
        let bound = BoundRosterSourceViewV1::capture(
            dataset,
            profile,
            objects,
            &claims,
            observations.clone(),
            captured_at,
            &mut grant,
        )?;
        let (pinned, family) =
            super::super::source_authority::assess(dataset, profile, &bound, &mut grant)?;
        let descriptor = finish_descriptor(
            dataset,
            snapshot.sequence(),
            profile,
            objects,
            profile_object,
            observations,
            &bound,
            &family,
        )?;
        Ok((descriptor, pinned))
    })();
    if result.is_err() {
        grant.poison();
    }
    result
}

fn finish_descriptor(
    dataset: &ExportDataset,
    snapshot_sequence: u64,
    profile: &QueryProfileV1,
    objects: &Path,
    profile_object: ObjectRef,
    observations: physical_observations::PhysicalObservationDescriptor,
    bound: &BoundRosterSourceViewV1<'_>,
    family: &super::super::source_authority::SourceAuthorityFamilyInventoryV1,
) -> ReportResult<FrozenAffiliationDescriptor> {
    let digest = proof_objects::install(objects, family.bytes())?;
    if digest != family.digest() {
        return Err(invalid("reviewed roster family object differs").into());
    }
    let family = SourceBindingObjectV1 {
        digest,
        bytes: u64::try_from(family.bytes().len()).map_err(|_| StoreError::CounterOverflow)?,
    };
    let closure = bound.closure_reference();
    let descriptor = FrozenAffiliationDescriptor {
        version: descriptor_version(profile)?,
        store_identity: dataset.lineage.store_identity.clone(),
        snapshot_sequence,
        input_generation: dataset.lineage.input_generation.clone(),
        input_digest: dataset.lineage.input_digest.clone(),
        source_digest: dataset.lineage.source_digest.clone(),
        profile_id: profile.id().map_err(StoreError::from)?,
        as_of: profile.input().as_of.clone(),
        profile: profile_object,
        observations,
        archives: ObjectRef {
            digest: closure.digest.clone(),
            bytes: closure.bytes,
        },
        roster_family: Some(family),
    };
    descriptor.validate(dataset, descriptor.observations.fingerprint())?;
    Ok(descriptor)
}

pub(super) fn replay(
    dataset: &ExportDataset,
    objects: &Path,
    descriptor: &FrozenAffiliationDescriptor,
    profile: &QueryProfileV1,
) -> ReportResult<PinnedCurrentAffiliations> {
    if descriptor.version != descriptor_version(profile)? {
        return Err(invalid("reviewed roster descriptor and profile versions differ").into());
    }
    let mut grant = AttachmentReplayGrantV1::new();
    let result = (|| {
        let closure = SourceBindingObjectV1 {
            digest: descriptor.archives.digest.clone(),
            bytes: descriptor.archives.bytes,
        };
        let family = descriptor
            .roster_family
            .as_ref()
            .ok_or_else(|| invalid("reviewed roster family root missing"))?;
        BoundRosterSourceViewV1::with_replay(
            dataset,
            profile,
            objects,
            &closure,
            &descriptor.observations,
            family,
            &mut grant,
            |bound, grant| {
                let (pinned, _) = super::super::source_authority::assess_historical(
                    dataset, profile, bound, grant,
                )?;
                Ok(pinned)
            },
        )
    })();
    if result.is_err() {
        grant.poison();
    }
    result
}

pub(super) fn descriptor_version(profile: &QueryProfileV1) -> ReportResult<u32> {
    use census_domain::profile_execution::{
        validate_selected_qualification_profile_v4, validate_selected_qualification_profile_v5,
    };
    let version = match profile.input().policies.affiliation.revision().get() {
        5 => {
            let _checked = validate_selected_qualification_profile_v4(profile)
                .map_err(|_| invalid("reviewed roster profile differs"))?;
            2
        }
        6 => {
            let _checked = validate_selected_qualification_profile_v5(profile)
                .map_err(|_| invalid("reviewed secondary-person profile differs"))?;
            3
        }
        _ => return Err(invalid("unsupported reviewed roster profile").into()),
    };
    Ok(version)
}
