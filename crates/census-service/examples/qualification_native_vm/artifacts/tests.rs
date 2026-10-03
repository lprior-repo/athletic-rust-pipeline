use super::*;
use std::io::{Seek, SeekFrom};

#[test]
fn append_checks_final_encoded_length_and_preserves_full_ledger_on_overrun() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("ledger.jsonl");
    let initial = LIMIT
        .checked_sub(2)
        .context("artifact fixture length underflow")?;
    let file = File::create(&path)?;
    file.set_len(initial)?;
    append(&path, &0_u8)?;
    assert_eq!(std::fs::metadata(&path)?.len(), LIMIT);
    assert!(append(&path, &0_u8).is_err());
    assert_eq!(std::fs::metadata(&path)?.len(), LIMIT);
    let mut file = File::open(&path)?;
    file.seek(SeekFrom::End(-2))?;
    let mut terminal = [0_u8; 2];
    file.read_exact(&mut terminal)?;
    assert_eq!(&terminal, b"0\n");
    assert_eq!(u64::try_from(read(&path)?.len())?, LIMIT);
    Ok(())
}

#[test]
fn publication_refuses_oversized_bytes_before_creating_an_artifact() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("oversized.bin");
    let length = usize::try_from(LIMIT.checked_add(1).context("artifact fixture overflow")?)?;
    let bytes = vec![0_u8; length];
    assert!(write(&path, &bytes).is_err());
    assert!(!path.exists());
    Ok(())
}

#[test]
fn publication_bounds_serialized_length_including_json_quotes() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("value.json");
    let length = usize::try_from(LIMIT.checked_sub(2).context("artifact fixture underflow")?)?;
    let at_limit = "a".repeat(length);
    publish(&path, &at_limit)?;
    assert_eq!(std::fs::metadata(&path)?.len(), LIMIT);
    let over_limit = "b".repeat(length.checked_add(1).context("artifact fixture overflow")?);
    assert!(publish(&path, &over_limit).is_err());
    assert!(!path.with_extension("pending").exists());
    assert_eq!(json(&path)?.as_str(), Some(at_limit.as_str()));
    Ok(())
}
