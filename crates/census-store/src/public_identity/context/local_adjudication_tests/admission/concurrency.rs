use super::*;
use crate::public_identity::graph_operation::preparation_concurrency::{exercise, PreparedBytes};

#[test]
fn concurrent_manual_preparations_adopt_original_timestamp_record_and_approval() -> TestResult {
    let fixture = merged()?;
    let request = request(&fixture)?;
    let head = fixture.writer.head(&fixture.store)?;
    exercise(&fixture.store, "manual-distinct-timestamps", || {
        let prepared = fixture.writer.prepare_local_adjudication(
            &fixture.store,
            "manual-distinct-timestamps",
            request.clone(),
            fixture.store.snapshot().revision(),
            &head,
        )?;
        Ok(PreparedBytes {
            record: serde_json::to_vec(prepared.record())?,
            approval: prepared.approval_bytes()?,
            digest: prepared.approval_digest()?,
        })
    })
}
