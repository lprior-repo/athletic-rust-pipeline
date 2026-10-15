use super::{physical, row, Rows, TestResult, Value, HEADER, PREFIX};

pub(super) fn verify(journal: &Rows) -> TestResult {
    let header = row(journal, HEADER)?;
    check!(eq; header["version"].as_u64(), Some(2));
    check!(eq; header["policy"].as_str(), Some("manual-context-admission/v2"));
    let mut bytes = 4096u64;
    let mut roots = 0u64;
    let mut reservations = 0u64;
    for (key, raw) in journal.iter().filter(|(key, _)| key.starts_with(PREFIX)) {
        bytes = bytes
            .checked_add(u64::try_from(
                key.len().checked_add(raw.len()).ok_or("metadata bytes")?,
            )?)
            .ok_or("admission bytes")?;
        reservations = reservations.checked_add(1).ok_or("admission count")?;
        let reservation: Value = serde_json::from_slice(raw)?;
        match reservation["version"].as_u64() {
            Some(3) => {
                check!(eq; reservation["purpose"]["capture"]["store"], header["store"]);
                let descriptors = reservation["descriptors"].as_array().ok_or("descriptors")?;
                check!(!descriptors.is_empty() && descriptors.len() <= 4096);
                let mut unique = std::collections::BTreeSet::new();
                for descriptor in descriptors {
                    let charge = &descriptor["incremental"];
                    let root = charge["root"].as_str().ok_or("charge root")?;
                    check!(unique.insert(root));
                    let amount = charge["bytes"].as_u64().ok_or("charged bytes")?;
                    let count = charge["roots"].as_u64().ok_or("charged roots")?;
                    check!(count <= 1 && (count == 0) == (amount == 0));
                    let complete = journal.contains_key(&super::key("manifest", root, None)?);
                    check!(count == 1 || complete);
                    physical::verify(journal, descriptor, complete)?;
                    if count == 1 {
                        check!(eq; charge["bytes"], descriptor["physical"]["full_bytes"]);
                    }
                    bytes = bytes.checked_add(amount).ok_or("physical charge")?;
                    roots = roots.checked_add(count).ok_or("root charge")?;
                }
            }
            Some(4) => {
                check!(eq; reservation["store"], header["store"]);
                let operation = reservation["raw_operation"]
                    .as_str()
                    .ok_or("raw operation")?;
                let graph = row(journal, &super::reservation_key(operation))?;
                check!(eq; graph["version"].as_u64(), Some(3));
                check!(eq; graph["purpose"]["capture"], reservation["graph"]["capture"]);
                check!(eq; graph["purpose"]["seal"], reservation["graph"]["seal"]);
                let root = reservation["graph"]["seal"]["root"]
                    .as_str()
                    .ok_or("seal root")?;
                check!(journal.contains_key(&super::key("manifest", root, None)?));
            }
            version => {
                return Err(format!("unexpected current admission version {version:?}").into())
            }
        }
    }
    check!(eq; header["bytes"].as_u64(), Some(bytes));
    check!(eq; header["roots"].as_u64(), Some(roots));
    check!(eq; header["reservations"].as_u64(), Some(reservations));
    check!(bytes <= 128 * 1024 * 1024 && roots <= 4096 && reservations <= 4096);
    Ok(())
}
