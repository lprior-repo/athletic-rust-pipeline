use super::{key, Rows, TestResult, Value, HEADER};

const PREFIX: &[u8] = b"manual-context-admission-v1\0reservation\0";

pub(super) fn verify(journal: &Rows, expected_reservations: u64) -> TestResult {
    let header: Value = serde_json::from_slice(journal.get(HEADER).ok_or("admission header")?)?;
    check!(eq; header["version"].as_u64(), Some(2));
    check!(eq; header["policy"].as_str(), Some("manual-context-admission/v2"));
    let mut bytes = 4096u64;
    let mut roots = 0u64;
    let mut reservations = 0u64;
    let mut kinds = std::collections::BTreeSet::new();
    for (key, raw) in journal.iter().filter(|(key, _)| key.starts_with(PREFIX)) {
        let reservation: Value = serde_json::from_slice(raw)?;
        check!(eq; reservation["version"].as_u64(), Some(2));
        check!(eq; reservation["decision"]["record"]["store"], header["store"]);
        check!(kinds.insert(
            reservation["decision"]["kind"]
                .as_str()
                .ok_or("kind")?
                .to_owned()
        ));
        bytes = bytes
            .checked_add(u64::try_from(
                key.len().checked_add(raw.len()).ok_or("row bytes")?,
            )?)
            .ok_or("bytes")?;
        reservations = reservations.checked_add(1).ok_or("reservation count")?;
        let charges = reservation["charges"].as_array().ok_or("charges")?;
        check!(eq; charges.len(), 2);
        for charge in charges {
            let amount = charge["bytes"].as_u64().ok_or("charged bytes")?;
            let count = charge["roots"].as_u64().ok_or("charged roots")?;
            check!(count <= 1 && (count == 0) == (amount == 0));
            retained(journal, charge, amount, count)?;
            bytes = bytes.checked_add(amount).ok_or("total bytes")?;
            roots = roots.checked_add(count).ok_or("total roots")?;
        }
    }
    check!(eq; reservations, expected_reservations);
    check!(eq; header["bytes"].as_u64(), Some(bytes));
    check!(eq; header["roots"].as_u64(), Some(roots));
    check!(eq; header["reservations"].as_u64(), Some(reservations));
    check!(bytes <= 128 * 1024 * 1024 && roots <= 4096 && reservations <= 4096);
    if expected_reservations == 2 {
        check!(kinds.contains("SplitV2") && kinds.contains("MembershipWithholdingV1"));
    }
    Ok(())
}

fn retained(journal: &Rows, charge: &Value, amount: u64, count: u64) -> TestResult {
    let root = charge["root"].as_str().ok_or("charged root")?;
    let manifest = key("manifest", root, None);
    if count == 0 {
        check!(journal.contains_key(&manifest));
        return Ok(());
    }
    let prefixes = [
        format!("j2\0chunk\0{root}\0"),
        format!("j2\0index\0{root}\0"),
    ];
    let total = journal
        .iter()
        .filter(|(key, _)| {
            key.as_slice() == manifest.as_slice()
                || prefixes
                    .iter()
                    .any(|prefix| key.starts_with(prefix.as_bytes()))
        })
        .try_fold(0u64, |total, (key, raw)| -> TestResult<u64> {
            let bytes = u64::try_from(key.len().checked_add(raw.len()).ok_or("physical size")?)?;
            total
                .checked_add(bytes)
                .ok_or_else(|| "physical bytes overflow".into())
        })?;
    check!(total <= amount);
    if journal.contains_key(&manifest) {
        check!(eq; total, amount);
    } else {
        check!(total < amount);
    }
    Ok(())
}
