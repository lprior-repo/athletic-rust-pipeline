use super::*;

impl<'a> BoundRosterSourceViewV1<'a> {
    pub(in super::super) fn replay(
        dataset: &'a ExportDataset,
        profile: &QueryProfileV1,
        objects: &'a Path,
        closure_object: &SourceBindingObjectV1,
        expected_athletes: &physical_observations::PhysicalObservationDescriptor,
        expected_family: &SourceBindingObjectV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> ReportResult<Self> {
        let result = (|| {
            dataset.ensure_input_binding()?;
            super::super::validate_profile(dataset, profile)?;
            let bytes = object::read(objects, closure_object, 64 * 1024, grant)?;
            let closure: RosterSourceClosureV1 =
                serde_json::from_slice(&bytes).map_err(|source| StoreError::Decode {
                    key: closure_object.digest.clone(),
                    source,
                })?;
            if closure.version != 1
                || &closure.athletes != expected_athletes
                || closure.reviewed.store_identity().as_str() != dataset.lineage.store_identity
                || closure.reviewed.selected_snapshot_sequence()
                    != dataset.lineage.snapshot_sequence
                || closure.acquisition_cutoff > closure.captured_at
            {
                return Err(invalid("reviewed roster frozen closure binding differs").into());
            }
            let lineage = object::read(objects, &closure.lineage, 64 * 1024, grant)?;
            let retained: crate::export::DatasetLineage = serde_json::from_slice(&lineage)
                .map_err(|source| StoreError::Decode {
                    key: closure.lineage.digest.clone(),
                    source,
                })?;
            if retained != dataset.lineage {
                return Err(invalid("reviewed roster frozen lineage differs").into());
            }
            closure
                .athletes
                .validate(dataset, closure.athletes.fingerprint(), budget())?;
            let source_ref = SourceBindingObjectV1 {
                digest: closure.reviewed.digest().into(),
                bytes: closure.reviewed.bytes(),
            };
            let source = object::read(objects, &source_ref, 4 * 1024 * 1024, grant)?;
            let claims = census_store::replay_frozen_roster_reviewed_claims_v1(
                &source,
                &closure.reviewed,
                grant,
            )?;
            if claims.source_store_lifecycle() != 8 {
                return Err(invalid("reviewed roster frozen source lifecycle differs").into());
            }
            let bound = Self {
                dataset,
                objects,
                claims: Claims::Frozen(claims),
                closure,
                closure_object: closure_object.clone(),
                expected_family: Some(expected_family.clone()),
            };
            bound.require_binding(dataset, profile, grant)?;
            Ok(bound)
        })();
        if result.is_err() {
            grant.poison();
        }
        result
    }
}
