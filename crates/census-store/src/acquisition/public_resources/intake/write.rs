use super::{codec, read, refused, Mint, PreparedPublicSchoolIntakeV1, PHASE};
use crate::acquisition::AcquisitionReadBudget;
use crate::{Application, PublicResourceCaptureOwnerV1, Receipt, Store, StoreError, StoreResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::AcquisitionOccurrenceRefV1;
use fjall::Readable;

pub(super) mod physical;

impl Store {
    pub fn prepare_public_school_intake(
        &self,
        owner: &PublicResourceCaptureOwnerV1,
        directory_original: &AcquisitionOccurrenceRefV1,
        roster_original: &AcquisitionOccurrenceRefV1,
    ) -> StoreResult<PreparedPublicSchoolIntakeV1> {
        let _writes = self.lock_appends();
        Ok(PreparedPublicSchoolIntakeV1 {
            owner: owner.clone(),
            packet: read::derive(self, owner, directory_original, roster_original)?,
        })
    }

    pub fn apply_public_school_intake(
        &self,
        prepared: PreparedPublicSchoolIntakeV1,
    ) -> StoreResult<Application> {
        let _writes = self.lock_appends();
        let packet = read::derive(
            self,
            &prepared.owner,
            prepared.packet.directory().occurrence(),
            prepared.packet.roster().occurrence(),
        )?;
        let bytes = packet.encode()?;
        if bytes != prepared.packet.encode()? {
            return Err(refused(
                "public school intake source changed after preparation",
            ));
        }
        let mint = Mint::from_packet(&packet)?;
        let key = mint.packet.as_str();
        let root = self.snapshot_locked().read_retained_capture_root_v1(
            crate::RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1,
            key,
            &mut AttachmentReplayGrantV1::new(),
        )?;
        if let Some(root) = root {
            if root.payload().get().as_bytes() != bytes {
                return Err(refused("public school intake existing packet differs"));
            }
            let admission = super::admission::read(
                &self.db.snapshot(),
                &self.meta,
                &self.journal,
                &self.receipts,
                &packet,
                &mut AcquisitionReadBudget::for_work(),
            )?;
            let receipt = codec::decode(admission.receipt_bytes())?;
            crate::acquisition::codec::repeat(self)?;
            return Ok(Application::Repeated(receipt));
        }
        let receipt = Receipt {
            operation: format!("{}{key}", super::OPERATION_PREFIX),
            digest: codec::object_digest(&mint)?.as_str().to_string(),
            at: chrono::Utc::now().to_rfc3339(),
            appended: 0,
            credited: false,
        };
        mint.validate(&packet, &receipt)?;
        self.commit_school_intake(&mint, &receipt, &bytes)
    }

    fn commit_school_intake(
        &self,
        mint: &Mint,
        receipt: &Receipt,
        bytes: &[u8],
    ) -> StoreResult<Application> {
        let snapshot = self.db.snapshot();
        if snapshot
            .contains_key(&self.meta, mint.key().as_bytes())
            .map_err(|source| StoreError::Read { source })?
            || snapshot
                .contains_key(&self.receipts, receipt.operation.as_bytes())
                .map_err(|source| StoreError::Read { source })?
            || snapshot
                .prefix(
                    &self.journal,
                    Store::journal_key(PHASE, mint.packet.as_str()),
                )
                .next()
                .is_some()
        {
            return Err(refused("public school intake incomplete existing mint"));
        }
        let records = physical::records(mint.packet.as_str(), &receipt.at, bytes)?;
        let mut batch = self.db.batch();
        for (key, value) in records {
            batch.insert(&self.journal, key, value);
        }
        batch.insert(&self.meta, mint.key().as_bytes(), codec::encode(mint)?);
        batch.insert(
            &self.receipts,
            receipt.operation.as_bytes(),
            codec::encode(receipt)?,
        );
        crate::acquisition::codec::commit(batch)?;
        Ok(Application::Written(receipt.clone()))
    }
}
