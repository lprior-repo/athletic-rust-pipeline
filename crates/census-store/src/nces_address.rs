use census_domain::model::{serialized_digest, NcesAddressRunReceiptV1, RunManifest};
use fjall::{PersistMode, Readable};

use crate::{Store, StoreError, StoreResult};

const RECEIPT_KEY: &str = "nces-address-run-receipt-v1";

impl Store {
    /// Persist immutable run-bound lineage for the selected run's CCD/PSS address generation.
    ///
    /// The school-address replay and its raw input archive live below `out/nces-address`; this
    /// compact record binds those retained artifacts to the current RunManifest without changing
    /// the historical RunManifest wire format.
    pub fn bind_nces_address_run_v1(&self, receipt: &NcesAddressRunReceiptV1) -> StoreResult<()> {
        let _guard = self.lock_appends();
        let snapshot = self.db.snapshot();
        let run: RunManifest =
            crate::run::read(&snapshot, &self.meta)?.ok_or_else(|| StoreError::Refused {
                detail: "NCES address receipt requires a bound census run".to_string(),
            })?;
        receipt
            .validate_for_run(&run)
            .map_err(|error| StoreError::Refused {
                detail: error.to_string(),
            })?;
        let run_digest = serialized_digest(&run)?;
        if receipt.run_manifest_digest() != run_digest {
            return Err(StoreError::Refused {
                detail: "NCES address receipt belongs to a different run manifest".to_string(),
            });
        }

        let encoded = serde_json::to_vec(receipt).map_err(|source| StoreError::Json {
            detail: "serializing NCES address receipt".to_string(),
            source,
        })?;
        if encoded.len() > crate::MAX_JOURNAL_VALUE_BYTES {
            return Err(StoreError::RowTooLarge {
                bytes: encoded.len(),
                max: crate::MAX_JOURNAL_VALUE_BYTES,
            });
        }
        if let Some(existing) = self
            .meta
            .get(RECEIPT_KEY)
            .map_err(|source| StoreError::Read { source })?
        {
            if existing.as_ref() == encoded.as_slice() {
                return Ok(());
            }
            return Err(StoreError::Refused {
                detail: "an immutable NCES address receipt is already bound to this store".into(),
            });
        }
        let mut batch = self.db.batch();
        batch.insert(&self.meta, RECEIPT_KEY, encoded);
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }

    /// Read and revalidate the immutable NCES receipt against the Store's current run binding.
    pub fn nces_address_run_v1(&self) -> StoreResult<Option<NcesAddressRunReceiptV1>> {
        let snapshot = self.db.snapshot();
        let Some(raw) = snapshot
            .get(&self.meta, RECEIPT_KEY)
            .map_err(|source| StoreError::Read { source })?
        else {
            return Ok(None);
        };
        if raw.len() > crate::MAX_JOURNAL_VALUE_BYTES {
            return Err(StoreError::RowTooLarge {
                bytes: raw.len(),
                max: crate::MAX_JOURNAL_VALUE_BYTES,
            });
        }
        let receipt: NcesAddressRunReceiptV1 =
            serde_json::from_slice(&raw).map_err(|source| StoreError::Json {
                detail: "stored NCES address receipt is invalid".to_string(),
                source,
            })?;
        let run: RunManifest =
            crate::run::read(&snapshot, &self.meta)?.ok_or_else(|| StoreError::Refused {
                detail: "stored NCES address receipt has no run manifest".to_string(),
            })?;
        receipt
            .validate_for_run(&run)
            .map_err(|error| StoreError::Refused {
                detail: error.to_string(),
            })?;
        if receipt.run_manifest_digest() != serialized_digest(&run)? {
            return Err(StoreError::Refused {
                detail: "stored NCES address receipt is bound to another run manifest".into(),
            });
        }
        Ok(Some(receipt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::{model::*, UsJurisdiction};

    fn manifest() -> TestResult<RunManifest> {
        let season = SchoolYear::new(2026).ok_or("school year")?;
        Ok(RunManifest {
            store_identity: "a".repeat(64),
            run: CensusRun::new(season, 3).ok_or("run")?,
            cohort: GradYear::CO2027,
            jurisdictions: vec![
                UsJurisdiction::Michigan,
                UsJurisdiction::Minnesota,
                UsJurisdiction::Iowa,
                UsJurisdiction::Illinois,
                UsJurisdiction::Colorado,
            ],
        })
    }

    fn receipt(run: &RunManifest) -> TestResult<NcesAddressRunReceiptV1> {
        Ok(NcesAddressRunReceiptV1::new(
            serialized_digest(run)?,
            run.jurisdictions.clone(),
            NcesArtifactReferenceV1::new(
                NcesArtifactProviderV1::Ccd,
                "b".repeat(64),
                128,
                "https://nces.ed.gov/example/ccd.csv",
                "2026-10-19",
            )?,
            NcesArtifactReferenceV1::new(
                NcesArtifactProviderV1::Pss,
                "c".repeat(64),
                256,
                "https://nces.ed.gov/example/pss.csv",
                "2026-10-19",
            )?,
            "d".repeat(64),
            "e".repeat(64),
        )?)
    }

    #[test]
    fn nces_receipt_is_bound_to_the_existing_run_and_survives_store_reopen() -> TestResult {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let run = manifest()?;
        store.bind_run(&run)?;
        let receipt = receipt(&run)?;

        store.bind_nces_address_run_v1(&receipt)?;
        check!(eq; store.nces_address_run_v1()?, Some(receipt.clone()));
        drop(store);

        let reopened = Store::open(dir.path())?;
        check!(eq; reopened.run_manifest()?, Some(run));
        check!(eq; reopened.nces_address_run_v1()?, Some(receipt));
        check!(reopened.integrity()?.ok);
        Ok(())
    }

    #[test]
    fn nces_receipt_with_wrong_run_digest_is_refused_without_a_receipt_write() -> TestResult {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let run = manifest()?;
        store.bind_run(&run)?;
        let valid = receipt(&run)?;
        let wrong = NcesAddressRunReceiptV1::new(
            "f".repeat(64),
            run.jurisdictions.clone(),
            valid.ccd().clone(),
            valid.pss().clone(),
            valid.input_manifest_digest(),
            valid.address_generation_digest(),
        )?;

        check!(store.bind_nces_address_run_v1(&wrong).is_err());
        check!(eq; store.nces_address_run_v1()?, None);
        Ok(())
    }

    #[test]
    fn conflicting_nces_receipt_is_refused_without_replacing_the_original() -> TestResult {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let run = manifest()?;
        store.bind_run(&run)?;
        let original = receipt(&run)?;
        store.bind_nces_address_run_v1(&original)?;
        let conflict = NcesAddressRunReceiptV1::new(
            serialized_digest(&run)?,
            run.jurisdictions.clone(),
            NcesArtifactReferenceV1::new(
                NcesArtifactProviderV1::Ccd,
                "f".repeat(64),
                128,
                "https://nces.ed.gov/example/other-ccd.csv",
                "2026-10-20",
            )?,
            original.pss().clone(),
            original.input_manifest_digest(),
            original.address_generation_digest(),
        )?;

        check!(store.bind_nces_address_run_v1(&conflict).is_err());
        check!(eq; store.nces_address_run_v1()?, Some(original));
        Ok(())
    }
}

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
