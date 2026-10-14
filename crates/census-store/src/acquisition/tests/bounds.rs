use super::*;

#[test]
fn maximal_original_and_revalidation_join_uses_eighty_three_roots_with_default_eighty_four(
) -> TestResult {
    let (_dir, store, writer, original_owner) = fixture()?;
    let original = maximal_body_work(&store, &writer, &original_owner)?;
    let current_owner = support::owner(&store, &writer, 2025, CaptureCacheIntent::AcquireNew)?;
    for status in [500, 429] {
        let observed = five_dispatch_response(
            &store,
            &writer,
            &current_owner,
            status,
            ConditionalRequestV1::Unconditional,
        )?;
        retain_response(&store, observed, status)?;
    }
    let conditional = ConditionalRequestV1::Pinned {
        original: original.clone(),
        etag: Some("bounded-fixture".to_string()),
        last_modified: None,
    };
    let response = five_dispatch_response(&store, &writer, &current_owner, 304, conditional)?;
    let revalidation = store.finish_acquisition_not_modified(
        response,
        &original,
        &mut AcquisitionReadBudget::for_work(),
    )?;
    let mut exact = AcquisitionReadBudget::new(83, 8 * 1024 * 1024, 64 * 1024 * 1024)?;
    let verified = store.read_retained_acquisition_v1(&revalidation, &mut exact)?;
    check!(eq; verified.body_reference(), &original);
    check!(eq; verified.revalidation().map(ResponseObservationV1::status), Some(304));
    check!(store
        .read_retained_acquisition_v1(&revalidation, &mut AcquisitionReadBudget::for_work())
        .is_ok());
    let mut short = AcquisitionReadBudget::new(82, 8 * 1024 * 1024, 64 * 1024 * 1024)?;
    check!(matches!(
        store.read_retained_acquisition_v1(&revalidation, &mut short),
        Err(StoreError::Capture(CaptureError::ReadBudgetExceeded {
            resource: "roots"
        }))
    ));
    Ok(())
}

fn maximal_body_work(
    store: &Store,
    writer: &CaptureWriterV1,
    owner: &ResultsCaptureOwnerV1,
) -> TestResult<AcquisitionOccurrenceRefV1> {
    let mut retained = None;
    for status in [500, 429, 200] {
        let observed = five_dispatch_response(
            store,
            writer,
            owner,
            status,
            ConditionalRequestV1::Unconditional,
        )?;
        retained = Some(retain_response(store, observed, status)?);
    }
    retained.ok_or_else(|| "missing original".into())
}

fn five_dispatch_response(
    store: &Store,
    writer: &CaptureWriterV1,
    owner: &ResultsCaptureOwnerV1,
    status: u16,
    conditional: ConditionalRequestV1,
) -> TestResult<ObservedResponse> {
    let reserved = store.reserve_acquisition_attempt(registered(store, writer, owner)?)?;
    let mut dispatch = store
        .arm_acquisition_dispatch(reserved, DispatchRequestV1::new(URL, conditional.clone())?)?
        .into_observation();
    let mut current = URL.to_string();
    for hop in 1..=4 {
        let target = format!("{URL}&hop={hop}");
        let response = store.record_acquisition_response(
            dispatch,
            ResponseObservationV1::new(
                302,
                &current,
                "2026-10-14T05:00:00Z",
                vec![("location".to_string(), target.clone())],
            )?,
        )?;
        dispatch = store
            .arm_acquisition_redirect(
                response,
                DispatchRequestV1::new(&target, conditional.clone())?,
            )?
            .into_observation();
        current = target;
    }
    Ok(store.record_acquisition_response(
        dispatch,
        ResponseObservationV1::new(
            status,
            &current,
            "2026-10-14T05:00:00Z",
            vec![("etag".to_string(), "bounded-fixture".to_string())],
        )?,
    )?)
}

fn retain_response(
    store: &Store,
    observed: ObservedResponse,
    status: u16,
) -> TestResult<AcquisitionOccurrenceRefV1> {
    let body = b"bounded original";
    let digest = format!("{:x}", Sha256::digest(body));
    let redirects: Vec<_> = (1..=4)
        .map(|hop| serde_json::json!({"status":302,"url":format!("{URL}&hop={hop}")}))
        .collect();
    let metadata = serde_json::to_vec_pretty(
        &serde_json::json!({"url":URL,"response_url":format!("{URL}&hop=4"),"method":"GET","status":status,"content_digest":digest,"bytes":body.len(),"fetched_at":"2026-10-14T05:00:00Z","etag":"bounded-fixture","redirects":redirects}),
    )?;
    let prepared = store.prepare_acquisition_capture(
        observed,
        PreparedCaptureInputV1::new(&digest, u64::try_from(body.len())?, metadata)?,
        CaptureDispositionV1::for_http_status(status)?,
    )?;
    publish(store, &prepared, body)?;
    Ok(store.finish_acquisition_capture(prepared)?)
}

#[derive(serde::Serialize)]
struct CodecEnvelope<'a> {
    key: &'a str,
    at: &'a str,
    payload: &'a str,
}

#[test]
fn maximum_canonical_envelope_and_physical_key_are_both_debited_before_materialization(
) -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let work = registered(&store, &writer, &owner)?;
    let key = format!("work/{}/attempt/3/dispatch/4/response", work.work_id());
    let today = "2026-10-14T00:00:00Z";
    let empty = serialized_bytes(&CodecEnvelope {
        key: &key,
        at: today,
        payload: "",
    })?;
    let payload = "a".repeat(
        MAX_CAPTURE_RECORD_BYTES
            .checked_sub(empty.len())
            .ok_or("fixed envelope too large")?,
    );
    let canonical = serialized_bytes(&CodecEnvelope {
        key: &key,
        at: today,
        payload: &payload,
    })?;
    check!(eq; canonical.len(), MAX_CAPTURE_RECORD_BYTES);
    let mut batch = store.db.batch();
    codec::stage(
        &store,
        &mut batch,
        &key,
        &payload,
        CaptureProtocolVersion::V1,
    )?;
    codec::commit(batch)?;
    let physical = Store::journal_key(SOURCE_ACQUISITION_PHASE, &key);
    let snapshot = store.db.snapshot();
    let mut value_only = AcquisitionReadBudget::new(1, MAX_CAPTURE_RECORD_BYTES, 0)?;
    check!(matches!(
        codec::raw(
            &snapshot,
            &store.journal,
            &key,
            &mut value_only,
            CaptureProtocolVersion::V1
        ),
        Err(StoreError::Capture(CaptureError::ReadBudgetExceeded {
            resource: "encoded bytes"
        }))
    ));
    let mut exact = AcquisitionReadBudget::new(
        1,
        MAX_CAPTURE_RECORD_BYTES
            .checked_add(physical.len())
            .ok_or("encoded budget")?,
        0,
    )?;
    let actual = codec::raw(
        &snapshot,
        &store.journal,
        &key,
        &mut exact,
        CaptureProtocolVersion::V1,
    )?
    .ok_or("missing staged envelope")?;
    check!(eq; actual.len(), MAX_CAPTURE_RECORD_BYTES);
    check!(eq; codec::decode::<String>(&key, &actual)?, payload.clone());
    let envelope: serde_json::Value = serde_json::from_slice(&actual)?;
    let retained_at = envelope
        .get("at")
        .and_then(serde_json::Value::as_str)
        .ok_or("missing retained timestamp")?;
    let canonical = serialized_bytes(&CodecEnvelope {
        key: &key,
        at: retained_at,
        payload: &payload,
    })?;
    check!(eq; actual, canonical);
    Ok(())
}

#[test]
fn oversized_canonical_envelope_refuses_before_any_journal_mutation() -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let work = registered(&store, &writer, &owner)?;
    let key = format!("work/{}/attempt/3/dispatch/4/response", work.work_id());
    let today = "2026-10-14T00:00:00Z";
    let empty = serialized_bytes(&CodecEnvelope {
        key: &key,
        at: today,
        payload: "",
    })?;
    let size = MAX_CAPTURE_RECORD_BYTES
        .checked_add(1)
        .and_then(|size| size.checked_sub(empty.len()))
        .ok_or("payload size")?;
    let payload = "a".repeat(size);
    check!(eq; serialized_bytes(&CodecEnvelope { key: &key, at: today, payload: &payload })?.len(), MAX_CAPTURE_RECORD_BYTES + 1);
    let before = rows(&store)?;
    let mut batch = store.db.batch();
    check!(matches!(
        codec::stage(
            &store,
            &mut batch,
            &key,
            &payload,
            CaptureProtocolVersion::V1
        ),
        Err(StoreError::RowTooLarge { .. })
    ));
    drop(batch);
    check!(eq; rows(&store)?, before);
    Ok(())
}
