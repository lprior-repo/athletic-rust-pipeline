use super::*;

#[derive(Clone, Copy)]
enum CaptureTamper {
    Digest,
    Body,
    Length,
}

fn tamper_capture(payload: &mut Value, family: Family, tamper: CaptureTamper) -> TestResult {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let capture = match family {
        Family::Provider => payload.get_mut("profile").ok_or("provider capture")?,
        Family::Html => payload,
        Family::Participation => payload.get_mut("status").ok_or("status capture")?,
    };
    match tamper {
        CaptureTamper::Digest => {
            *capture.get_mut("body_sha256").ok_or("original digest")? =
                Value::String("0".repeat(64));
        }
        CaptureTamper::Body => {
            let encoded = capture
                .get("body_base64")
                .and_then(Value::as_str)
                .ok_or("body encoding")?;
            let mut body = STANDARD.decode(encoded)?;
            *body.last_mut().ok_or("original body final byte")? ^= 1;
            *capture
                .get_mut("body_base64")
                .ok_or("retained encoded body")? = Value::String(STANDARD.encode(body));
        }
        CaptureTamper::Length => {
            *capture.get_mut("body_bytes").ok_or("original length")? = Value::from(1);
        }
    }
    Ok(())
}

#[test]
fn body_bytes_digest_and_length_tampering_refuse_before_any_checked_holder_escapes() -> TestResult {
    for (family, tamper) in FAMILIES.into_iter().flat_map(|family| {
        [
            CaptureTamper::Digest,
            CaptureTamper::Body,
            CaptureTamper::Length,
        ]
        .into_iter()
        .map(move |tamper| (family, tamper))
    }) {
        let (directory, store) = fresh_store()?;
        let (mut recorded, reference) = genuine_recorded(&store, family)?;
        let payload = &mut recorded.journal.first_mut().ok_or("original root")?.payload;
        tamper_capture(payload, family, tamper)?;
        let retained = install_for_refusal(recorded, &store, "tampered-source-body")?;
        drop(store);
        let store = reopen(&directory)?;
        let before = protected_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        let refused = read_present(&store, family, &reference, &mut grant);
        match tamper {
            CaptureTamper::Digest | CaptureTamper::Body => check!(matches!(refused,
                Err(CrawlError::Store(StoreError::Refused { detail }))
                if detail == "participation HTML body length or digest differs")),
            CaptureTamper::Length => check!(matches!(
                refused,
                Err(CrawlError::Store(StoreError::EvidenceSubjectAttachment(
                    EvidenceSubjectAttachmentStoreError::Corrupt(
                        "external capture base64 or digest shape"
                    )
                )))
            )),
        }
        require_retained_state(&store, &retained)?;
        require_poisoned(read_present(&store, family, &reference, &mut grant))?;
        require_retained_state(&store, &retained)?;
        check!(eq; protected_state(&store)?, before);
        no_public_birth(&store)?;
    }
    Ok(())
}
