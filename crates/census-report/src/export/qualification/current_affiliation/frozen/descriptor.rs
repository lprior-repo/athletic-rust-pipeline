use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct ObjectRef {
    pub(super) digest: String,
    pub(super) bytes: u64,
}

impl ObjectRef {
    pub(super) fn validate(&self, ceiling: u64) -> census_store::StoreResult<()> {
        if !super::super::note::digest(&self.digest) || self.bytes == 0 || self.bytes > ceiling {
            return Err(invalid("frozen affiliation object reference is invalid"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FrozenAffiliationDescriptor {
    pub(super) version: u32,
    pub(super) store_identity: String,
    pub(super) snapshot_sequence: u64,
    pub(super) input_generation: String,
    pub(super) input_digest: String,
    pub(super) source_digest: String,
    pub(super) profile_id: ProfileId,
    pub(super) as_of: ProfileDate,
    pub(super) profile: ObjectRef,
    pub(super) observations: physical_observations::PhysicalObservationDescriptor,
    pub(super) archives: ObjectRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) roster_family: Option<super::super::source_binding::SourceBindingObjectV1>,
}

impl FrozenAffiliationDescriptor {
    pub(crate) fn reviewed_roster(&self) -> bool {
        self.version == 2
    }

    pub(crate) fn digest(&self) -> census_store::StoreResult<String> {
        census_domain::model::serialized_digest(self).map_err(StoreError::from)
    }

    pub(super) fn require_digest(&self, expected: &str) -> census_store::StoreResult<()> {
        if !super::super::note::digest(expected) || self.digest()? != expected {
            return Err(invalid(
                "frozen affiliation descriptor differs from its externally bound root",
            ));
        }
        Ok(())
    }

    pub(crate) fn athlete_table_digest(&self) -> &str {
        self.observations.fingerprint()
    }

    pub(crate) fn observations(&self) -> &physical_observations::PhysicalObservationDescriptor {
        &self.observations
    }

    pub(super) fn validate(
        &self,
        dataset: &ExportDataset,
        expected: &str,
    ) -> census_store::StoreResult<()> {
        if !matches!(
            (self.version, self.roster_family.is_some()),
            (1, false) | (2, true)
        ) || self.store_identity != dataset.lineage.store_identity
            || self.snapshot_sequence != dataset.lineage.snapshot_sequence
            || self.input_generation != dataset.lineage.input_generation
            || self.input_digest != dataset.lineage.input_digest
            || self.source_digest != dataset.lineage.source_digest
            || Some(&self.profile_id) != dataset.selected_profile_id()
            || Some(&self.as_of) != dataset.selected_as_of()
            || self.observations.table() != census_store::Table::Athletes
            || census_domain::model::serialized_digest(dataset)? != self.input_digest
        {
            return Err(invalid(
                "frozen affiliation descriptor differs from its bound input or physical closure",
            ));
        }
        for digest in [
            &self.store_identity,
            &self.input_generation,
            &self.input_digest,
            &self.source_digest,
        ] {
            if !super::super::note::digest(digest) {
                return Err(invalid(
                    "frozen affiliation binding contains a noncanonical digest",
                ));
            }
        }
        self.profile.validate(PROFILE_BYTES)?;
        self.observations
            .validate(dataset, expected, observation_budget())?;
        self.archives.validate(INVENTORY_BYTES)?;
        if let Some(family) = &self.roster_family {
            ObjectRef {
                digest: family.digest.clone(),
                bytes: family.bytes,
            }
            .validate(8 * 1024 * 1024)?;
        }
        Ok(())
    }
}
