use super::*;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const URL: &str =
    "https://mi.milesplit.com/api/v1/meets/123/performances?isMeetPro=0&fields=fixture";

pub(super) fn fixture() -> TestResult<(
    tempfile::TempDir,
    Store,
    CaptureWriterV1,
    ResultsCaptureOwnerV1,
)> {
    fixture_for(CaptureProtocolVersion::V2)
}
pub(super) fn fixture_for(
    protocol: CaptureProtocolVersion,
) -> TestResult<(
    tempfile::TempDir,
    Store,
    CaptureWriterV1,
    ResultsCaptureOwnerV1,
)> {
    if protocol != CaptureProtocolVersion::V2 {
        return Err(CaptureError::CaptureProtocolUnsupported.into());
    }
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("store");
    Store::create_capture_qualification(&root)?;
    let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
    let run = CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("run")?;
    store.bind_run(&RunManifest {
        store_identity: writer.store_identity().as_str().to_string(),
        run,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::Michigan],
    })?;
    let owner = owner(&store, &writer, 2026, CaptureCacheIntent::ReuseEligible)?;
    Ok((temporary, store, writer, owner))
}
pub(super) fn owner(
    store: &Store,
    writer: &CaptureWriterV1,
    year: u16,
    intent: CaptureCacheIntent,
) -> TestResult<ResultsCaptureOwnerV1> {
    let manifest = store.run_manifest()?.ok_or("manifest")?;
    let scope = ResultsCaptureScopeV1::new(
        "jurisdiction:MI:2026-27:1",
        UsJurisdiction::Michigan,
        manifest.run,
        AcquisitionDigest::parse(&serialized_digest(&manifest)?)?,
        SelectedHistoryV1::new(HistoryWindow::new(2023, 2026, "2026-10-14")?, year)?,
        SourceAdmissionV1::milesplit_results(&"a".repeat(64))?,
        intent,
    )?;
    Ok(writer.results_owner(store, scope)?)
}
pub(super) fn work(owner: &ResultsCaptureOwnerV1) -> StoreResult<AcquisitionWorkV1> {
    owner.for_owned_meet(
        UsJurisdiction::Michigan,
        "123",
        RequestRepresentationV1::get(URL, vec![], "Census/1.0 (public-source qualification)")
            .map_err(CaptureError::Domain)?,
    )
}
pub(super) fn registered(
    store: &Store,
    writer: &CaptureWriterV1,
    owner: &ResultsCaptureOwnerV1,
) -> StoreResult<OwnedAcquisitionWork> {
    let grant =
        writer.continuation_for_owned_meet(store, owner, UsJurisdiction::Michigan, "123")?;
    store.register_acquisition_work(owner, work(owner)?, grant)
}
pub(super) fn dispatched(
    store: &Store,
    writer: &CaptureWriterV1,
    owner: &ResultsCaptureOwnerV1,
) -> TestResult<DispatchObservation> {
    let reserved = store.reserve_acquisition_attempt(registered(store, writer, owner)?)?;
    Ok(store
        .arm_acquisition_dispatch(
            reserved,
            DispatchRequestV1::new(URL, ConditionalRequestV1::Unconditional)?,
        )?
        .into_observation())
}
pub(super) fn prepared(
    store: &Store,
    writer: &CaptureWriterV1,
    owner: &ResultsCaptureOwnerV1,
    status: u16,
) -> TestResult<(PreparedCapture, Vec<u8>)> {
    let dispatch = dispatched(store, writer, owner)?;
    let response = ResponseObservationV1::new(
        status,
        URL,
        "2026-10-14T05:00:00Z",
        vec![("etag".to_string(), "fixture-etag".to_string())],
    )?;
    let observed = store.record_acquisition_response(dispatch, response)?;
    let body = b"retained body".to_vec();
    let digest = format!("{:x}", Sha256::digest(&body));
    let metadata = serde_json::to_vec_pretty(
        &serde_json::json!({"url":URL,"response_url":URL,"method":"GET","status":status,"content_digest":digest,"bytes":body.len(),"fetched_at":"2026-10-14T05:00:00Z","etag":"fixture-etag"}),
    )?;
    let input = PreparedCaptureInputV1::new(&digest, u64::try_from(body.len())?, metadata)?;
    Ok((
        store.prepare_acquisition_capture(
            observed,
            input,
            CaptureDispositionV1::for_http_status(status)?,
        )?,
        body,
    ))
}
pub(super) fn publish(
    store: &Store,
    prepared: &PreparedCapture,
    body: &[u8],
) -> TestResult<(std::path::PathBuf, std::path::PathBuf)> {
    let capture = prepared.prepared();
    let bodies = store.http_cache_dir().join("archive/bodies");
    let captures = store
        .http_cache_dir()
        .join("archive/captures")
        .join(capture.content_digest().as_str());
    crate::fs::create_dir_all_synced(&bodies)?;
    crate::fs::create_dir_all_synced(&captures)?;
    let body_path = bodies.join(format!("{}.body", capture.content_digest()));
    let meta_path = captures.join(format!("{}.meta.json", capture.metadata_digest()));
    std::fs::write(&body_path, body)?;
    std::fs::write(&meta_path, capture.metadata_bytes())?;
    Ok((body_path, meta_path))
}
pub(super) fn retained(
    store: &Store,
    writer: &CaptureWriterV1,
    owner: &ResultsCaptureOwnerV1,
    status: u16,
) -> TestResult<AcquisitionOccurrenceRefV1> {
    let (prepared, body) = prepared(store, writer, owner, status)?;
    publish(store, &prepared, &body)?;
    Ok(store.finish_acquisition_capture(prepared)?)
}
pub(super) fn rows(store: &Store) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut rows = Vec::new();
    for keyspace in [&store.meta, &store.journal, &store.receipts] {
        for guard in store.db.snapshot().prefix(keyspace, b"").take(1024) {
            let (key, value) = guard.into_inner()?;
            rows.push((key.to_vec(), value.to_vec()));
        }
    }
    Ok(rows)
}
pub(super) fn overwrite<T: serde::Serialize>(store: &Store, key: &str, payload: &T) -> TestResult {
    let encoded =
        serde_json::to_vec(&serde_json::json!({"key":key,"at":"2026-10-14","payload":payload}))?;
    store.journal.insert(
        Store::journal_key(CaptureProtocolVersion::V2.phase(), key),
        encoded,
    )?;
    store.flush()?;
    Ok(())
}
