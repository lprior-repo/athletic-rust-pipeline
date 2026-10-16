use super::*;

pub(super) fn phase(phase: &str, outcome: &str, detail: Value) {
    if report::phase("bridge", phase, outcome, detail).is_err() {
        eprintln!(
            "GENUINE_ATTACHMENT bridge-diagnostic-write-failed phase={phase} outcome={outcome}"
        );
    }
}

pub(super) fn failure(phase_name: &str, error: &(dyn std::error::Error + 'static)) {
    let store =
        error
            .downcast_ref::<StoreError>()
            .or_else(|| match error.downcast_ref::<CrawlError>() {
                Some(CrawlError::Store(error)) => Some(error),
                _ => None,
            });
    let detail = match store {
        Some(StoreError::EvidenceSubjectAttachment(
            EvidenceSubjectAttachmentStoreError::Domain(EvidenceSubjectAttachmentError::Capacity {
                resource,
                needed,
                available,
            }),
        )) => json!({"typed":"EvidenceSubjectAttachment::Domain::Capacity",
            "resource":resource, "needed":needed, "available":available}),
        Some(other) => json!({"typed":"StoreError",
            "category":format!("{:?}", std::mem::discriminant(other)),
            "detail":"Original typed error returned unchanged; no source payload emitted"}),
        None => json!({"typed":"checked-consumer-or-test-error",
            "detail":"Original error returned unchanged; no source payload emitted"}),
    };
    phase(phase_name, "refused", detail);
}

pub(super) fn preservation(observed: &TestResult) {
    phase(
        "preservation",
        if observed.is_ok() {
            "preserved"
        } else {
            "observation-failed"
        },
        json!({"checks":"Exact namespaces, logical payloads, receipts and installed accepted provider",
            "primary_error":"Original consumer failure remains the returned error"}),
    );
}
