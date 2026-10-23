use super::*;
use crate::journal::context::ContextPhysicalRef;
use serde::ser::SerializeSeq;

#[derive(serde::Serialize)]
struct DecoderEnvelope<'a> {
    version: u8,
    purpose: &'a Purpose,
    descriptors: RepeatedDescriptors<'a>,
}

struct RepeatedDescriptors<'a> {
    descriptor: &'a Descriptor,
    tail: &'a Descriptor,
    preceding: usize,
}

impl serde::Serialize for RepeatedDescriptors<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let count = self
            .preceding
            .checked_add(1)
            .ok_or_else(|| <S::Error as serde::ser::Error>::custom("descriptor count overflow"))?;
        let mut sequence = serializer.serialize_seq(Some(count))?;
        for _ in 0..self.preceding {
            sequence.serialize_element(self.descriptor)?;
        }
        sequence.serialize_element(self.tail)?;
        sequence.end()
    }
}

fn decoder_input(
    reservation: &GraphReservation,
    descriptor: &Descriptor,
    tail: &Descriptor,
    preceding: usize,
) -> TestResult<Vec<u8>> {
    Ok(serde_json::to_vec(&DecoderEnvelope {
        version: reservation.version,
        purpose: &reservation.purpose,
        descriptors: RepeatedDescriptors {
            descriptor,
            tail,
            preceding,
        },
    })?)
}

fn physical_ref(rows: usize) -> TestResult<ContextPhysicalRef> {
    let root = "a".repeat(64);
    let chunks = rows.checked_sub(2).ok_or("physical row count")?;
    let jsonl = chunks
        .checked_mul(crate::journal::OBJECT_CHUNK_BYTES)
        .ok_or("physical JSONL length")?;
    let manifest = u64::try_from(crate::journal::object_manifest_key(&root)?.len())?
        .checked_add(1)
        .ok_or("manifest length")?;
    let full = (0..chunks).try_fold(manifest, |total, ordinal| -> TestResult<u64> {
        let key = crate::journal::object_chunk_key(&root, ordinal)?;
        let bytes = u64::try_from(key.len())?
            .checked_add(u64::try_from(crate::journal::OBJECT_CHUNK_BYTES)?)
            .ok_or("chunk length")?;
        total.checked_add(bytes).ok_or_else(|| "chunk total".into())
    })?;
    let full = full
        .checked_add(u64::try_from(
            crate::journal::object_index_key(&root, 0)?.len(),
        )?)
        .and_then(|total| total.checked_add(1))
        .ok_or("index length")?;
    let row = serde_json::json!({"length": 1, "digest": vec![0u8; 32]});
    let chunk = serde_json::json!({
        "length": crate::journal::OBJECT_CHUNK_BYTES,
        "digest": vec![0u8; 32]
    });
    let physical: ContextPhysicalRef = serde_json::from_value(serde_json::json!({
        "root": root,
        "full_bytes": full,
        "jsonl_bytes": jsonl,
        "records": 1,
        "manifest": row,
        "chunks": vec![chunk; chunks],
        "pages": [row]
    }))?;
    check!(eq; physical.row_count(), rows);
    Ok(physical)
}

#[test]
fn decoder_accepts_exact_physical_row_bound_and_refuses_one_more() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (_generation, plan) = Generation::new(&store, 0)?;
    let reservation = reserve(&store, "physical-decoder", plan)?;
    let mut descriptor = reservation
        .descriptors
        .first()
        .ok_or("page descriptor")?
        .clone();
    descriptor.physical = physical_ref(128)?;
    let mut tail = descriptor.clone();
    let preceding = MAX_PHYSICAL_ROWS
        .checked_div(128)
        .ok_or("row quotient")?
        .checked_sub(1)
        .ok_or("preceding descriptor count")?;
    check!(eq; MAX_PHYSICAL_ROWS % 128, 0);
    check!(preceding.checked_add(1).ok_or("descriptor count")? < MAX_DESCRIPTORS);
    let before = retained_state(&store)?;
    let counters_before = counters(&store)?;
    let sequence = store.snapshot().sequence();
    let raw = decoder_input(&reservation, &descriptor, &tail, preceding)?;
    check!(raw.len() < MAX_METADATA_BYTES);
    let decoded: GraphReservation = serde_json::from_slice(&raw)?;
    let total = decoded
        .descriptors
        .iter()
        .try_fold(0usize, |total, value| {
            total
                .checked_add(value.physical.row_count())
                .ok_or("physical row total")
        })?;
    check!(eq; total, MAX_PHYSICAL_ROWS);
    check!(validate::validate(&decoded, &store.store_identity()?).is_err());
    drop(decoded);
    drop(raw);
    tail.physical = physical_ref(129)?;
    let raw = decoder_input(&reservation, &descriptor, &tail, preceding)?;
    check!(raw.len() < MAX_METADATA_BYTES);
    let error = serde_json::from_slice::<GraphReservation>(&raw)
        .err()
        .ok_or("physical row overflow accepted")?;
    check!(error
        .to_string()
        .contains("cumulative graph admission bound"));
    check!(eq; retained_state(&store)?, before);
    check!(eq; counters(&store)?, counters_before);
    check!(eq; store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn physical_fingerprint_allowance_fits_the_metadata_budget() -> TestResult {
    let rows = MAX_PHYSICAL_ROWS.checked_mul(256).ok_or("row metadata")?;
    let descriptors = MAX_DESCRIPTORS
        .checked_mul(512)
        .ok_or("descriptor metadata")?;
    let allowance = rows.checked_add(descriptors).ok_or("metadata total")?;
    check!(eq; allowance, 72 * 1024 * 1024);
    check!(allowance < MAX_METADATA_BYTES);
    Ok(())
}
