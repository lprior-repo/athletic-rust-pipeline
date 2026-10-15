use super::*;

pub(super) struct Fixture {
    pub(super) _dir: tempfile::TempDir,
    pub(super) store: Store,
    pub(super) fetcher: Fetcher,
    pub(super) reference: ResultSetRef,
    pub(super) owned: OwnedAcquisitionRef,
    pub(super) raw: RawMetadataArchiveRef,
}

impl Fixture {
    pub(super) async fn prepare(body: &[u8]) -> TestResult<Self> {
        let (dir, store, fetcher, reference) = setup()?;
        seed_owned(&fetcher, &reference, body)?;
        seed_metadata(&fetcher, &reference)?;
        let report = crate::milesplit::collect_result_sets(
            &context(&store, &fetcher)?,
            &options(&reference),
        )
        .await?;
        assert_eq!(report.errors, 0, "{:?}", report.notes);
        let receipts = store.journal_payloads(crate::milesplit::RESULT_SET_PHASE)?;
        let receipt = receipts
            .iter()
            .find(|value| {
                value.get("disposition").and_then(Value::as_str) == Some("projection_applied")
            })
            .ok_or("projection receipt")?;
        let owned = OwnedAcquisitionRef::parse(
            receipt
                .get("owned_capture_archive")
                .and_then(Value::as_str)
                .ok_or("owned archive")?,
        )?;
        let raw = RawMetadataArchiveRef::parse(
            receipt
                .get("raw_metadata_archive")
                .and_then(|value| value.get("archive_manifest"))
                .and_then(Value::as_str)
                .ok_or("raw archive")?,
        )?;
        Ok(Self {
            _dir: dir,
            store,
            fetcher,
            reference,
            owned,
            raw,
        })
    }

    pub(super) fn read(&self) -> TestResult<VerifiedRetainedResultMetadata> {
        self.read_archive(&self.raw)
    }

    pub(super) fn read_archive(
        &self,
        archive: &RawMetadataArchiveRef,
    ) -> TestResult<VerifiedRetainedResultMetadata> {
        let view = self.store.snapshot();
        let owned = read_retained_owned_result(&view, &self.owned, "data[0]", budget()?)?;
        Ok(read_retained_result_metadata(
            &view,
            &self.reference,
            archive,
            &owned,
            budget()?,
        )?)
    }

    pub(super) fn archive(
        &self,
        body: &[u8],
        response: Option<&str>,
    ) -> TestResult<RawMetadataArchiveRef> {
        let capture = raw_capture(body, &self.reference, response);
        super::super::super::capture::archive_metadata(
            &context(&self.store, &self.fetcher)?,
            &self.reference,
            &capture,
        )?;
        let metadata =
            super::super::super::capture::metadata_provenance(&self.reference, &capture)?;
        Ok(RawMetadataArchiveRef::parse(
            metadata
                .get("archive_manifest")
                .and_then(Value::as_str)
                .ok_or("archive key")?,
        )?)
    }

    pub(super) fn payload(&self, key: &str) -> TestResult<Value> {
        self.store
            .journal_payload(OWNED_CAPTURE_PHASE, key)?
            .ok_or_else(|| "journal payload".into())
    }

    pub(super) fn replace(&self, key: &str, payload: &Value) -> TestResult {
        let mut batch = self.store.write_batch();
        batch.journal_done(OWNED_CAPTURE_PHASE, key, payload)?;
        batch.commit()?;
        Ok(())
    }
}

pub(super) fn raw_capture(
    body: &[u8],
    reference: &ResultSetRef,
    response: Option<&str>,
) -> FetchOutcome {
    FetchOutcome {
        url: reference.url.clone(),
        response_url: response.map(str::to_owned),
        method: "GET".into(),
        status: 200,
        content_digest: crate::net::cache::content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-10-01T23:44:16Z".into(),
        from_cache: false,
        content_type: None,
        body: body.to_vec(),
    }
}

pub(super) fn budget() -> TestResult<CaptureReadBudget> {
    Ok(CaptureReadBudget::new(1024 * 1024)?)
}
