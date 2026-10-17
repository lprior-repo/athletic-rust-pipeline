use super::*;

impl<'a> BoundRosterSourceViewV1<'a> {
    pub(in super::super) fn capture(
        dataset: &'a ExportDataset,
        profile: &QueryProfileV1,
        objects: &'a Path,
        claims: &'a CurrentRosterReviewedClaimsV1,
        athletes: physical_observations::PhysicalObservationDescriptor,
        captured_at: DateTime<Utc>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> ReportResult<Self> {
        let result = (|| {
            dataset.ensure_input_binding()?;
            super::super::validate_profile(dataset, profile)?;
            claims.require_original_grant(grant)?;
            if claims.store_identity().as_str() != dataset.lineage.store_identity
                || claims.selected_snapshot_sequence() != dataset.lineage.snapshot_sequence
                || claims.source_store_lifecycle() != 8
            {
                return Err(
                    invalid("reviewed roster capture owner differs from publication").into(),
                );
            }
            athletes.validate(dataset, athletes.fingerprint(), budget())?;
            let bundle = claims.freeze_closure(grant)?;
            let installed = proof_objects::install(objects, bundle.bytes())?;
            if installed != bundle.digest() {
                return Err(invalid("reviewed roster installed bundle differs").into());
            }
            let lineage = object::encode(objects, &dataset.lineage, grant)?;
            let closure = RosterSourceClosureV1 {
                version: 1,
                lineage,
                reviewed: bundle.reference().clone(),
                athletes,
                captured_at,
                acquisition_cutoff: captured_at,
            };
            let closure_object = object::encode(objects, &closure, grant)?;
            let bound = Self {
                dataset,
                objects,
                claims: Claims::Current(claims),
                closure,
                closure_object,
                expected_family: None,
            };
            bound.require_binding(dataset, profile, grant)?;
            Ok(bound)
        })();
        if result.is_err() {
            grant.poison();
        }
        result
    }

    pub(in super::super) fn closure_reference(&self) -> &SourceBindingObjectV1 {
        &self.closure_object
    }

    pub(in super::super) fn athlete_descriptor(
        &self,
    ) -> &physical_observations::PhysicalObservationDescriptor {
        &self.closure.athletes
    }
}
