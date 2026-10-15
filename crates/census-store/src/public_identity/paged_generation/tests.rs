mod boundaries;
mod corruption;
mod hierarchy;
mod physical;

use super::*;
use census_domain::model::{ReviewVerdictRecord, StoreIdentity};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub(super) fn capture() -> TestResult<CaptureBindingV2> {
    Ok(CaptureBindingV2::new(
        StoreIdentity::parse(&"1".repeat(64))?,
        FormatBindingV2::new(1, 1, 1)?,
        RunBindingV2::Unbound,
        CapturePositionV2::new(9, 7, 12, [3, 4, 5, 6, 7, 8]),
    )?)
}

pub(super) fn verdict(id: &str) -> ReviewVerdictRecord {
    ReviewVerdictRecord {
        id: id.into(),
        case_id: id.into(),
        subject_id: "subject".into(),
        family: "identity".into(),
        kind: "rejected".into(),
        field: "identity".into(),
        value: "unknown".into(),
        accepted: false,
        confidence: 0,
        rationale: "é \\\" \\u0061".into(),
        reviewer: "fixture".into(),
        observed_at: "2026-10-15".into(),
        member_ids: Vec::new(),
    }
}

pub(super) fn row(
    binding: &TableBindingV2,
    id: &str,
    bytes: usize,
) -> TestResult<RawPhysicalRowV2> {
    let key = crate::keys::observation_key(crate::Table::IdentityVerdicts, id, 0);
    let mut value = serde_json::to_vec(&verdict(id))?;
    if bytes > value.len() {
        value.resize(bytes, b' ');
    }
    Ok(RawPhysicalRowV2::new(binding, &key, &value)?)
}

pub(super) fn page() -> TestResult<(TableBindingV2, RawPageV2)> {
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let page = RawPageV2::new(
        &binding,
        RawPagePositionV2::new(0, 0)?,
        vec![row(&binding, "a", 0)?],
    )?;
    Ok((binding, page))
}

pub(super) fn object(bytes: &[u8]) -> TestResult<ObjectRefV2> {
    let records = u32::try_from(bytes.iter().filter(|byte| **byte == b'\n').count())?;
    Ok(ObjectRefV2::new(
        DigestV2::of_bytes(bytes),
        u32::try_from(bytes.len())?,
        records,
        u64::try_from(bytes.len())?,
    )?)
}

pub(super) fn seal() -> TestResult<(CaptureBindingV2, RawGenerationSealV2)> {
    let binding = capture()?;
    let references = AuthorityTableV2::ALL
        .into_iter()
        .map(|table| {
            let table = RawTableManifestV2::new(&binding.table_binding(table)?, Vec::new())?;
            let bytes = table.encode_jsonl()?;
            table.reference(object(&bytes)?).map_err(Into::into)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let tables: [RawTableReferenceV2; 6] = references.try_into().map_err(|_| "table count")?;
    Ok((binding.clone(), RawGenerationSealV2::new(binding, tables)?))
}

pub(super) fn replace(bytes: &[u8], needle: &str, replacement: &str) -> TestResult<Vec<u8>> {
    let text = std::str::from_utf8(bytes)?;
    if !text.contains(needle) {
        return Err(format!("missing mutation target: {needle}").into());
    }
    Ok(text.replacen(needle, replacement, 1).into_bytes())
}
