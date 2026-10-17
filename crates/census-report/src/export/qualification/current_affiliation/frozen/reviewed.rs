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
            version: 2,
            store_identity: dataset.lineage.store_identity.clone(),
            snapshot_sequence: snapshot.sequence(),
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
        Ok((descriptor, pinned))
    })();
    if result.is_err() {
        grant.poison();
    }
    result
}

pub(super) fn replay(
    dataset: &ExportDataset,
    objects: &Path,
    descriptor: &FrozenAffiliationDescriptor,
    profile: &QueryProfileV1,
) -> ReportResult<PinnedCurrentAffiliations> {
    let _checked =
        census_domain::profile_execution::validate_selected_qualification_profile_v4(profile)
            .map_err(|_| invalid("reviewed roster replay profile differs"))?;
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
        let bound = BoundRosterSourceViewV1::replay(
            dataset,
            profile,
            objects,
            &closure,
            &descriptor.observations,
            family,
            &mut grant,
        )?;
        let (pinned, _) = super::super::source_authority::assess_historical(
            dataset, profile, &bound, &mut grant,
        )?;
        Ok(pinned)
    })();
    if result.is_err() {
        grant.poison();
    }
    result
}
