use super::*;

#[test]
fn append_rejects_final_encoded_size_without_mutating_the_ledger() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("ledger.jsonl");
    let file = std::fs::File::create(&path)?;
    file.set_len(MAX_ARTIFACT.checked_sub(1).context("limit underflow")?)?;
    let before = file.metadata()?.len();
    let error = append(&path, &json!({"record":"new"}))
        .err()
        .context("oversize append accepted")?;
    assert!(error.to_string().contains("resource-limit"));
    assert_eq!(file.metadata()?.len(), before);
    Ok(())
}

#[test]
fn append_counts_the_newline_and_accepts_exactly_the_reader_limit() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("ledger.jsonl");
    let file = std::fs::File::create(&path)?;
    file.set_len(MAX_ARTIFACT.checked_sub(5).context("limit underflow")?)?;
    append(&path, &Value::Null)?;
    assert_eq!(file.metadata()?.len(), MAX_ARTIFACT);
    let error = append(&path, &Value::Null)
        .err()
        .context("full ledger accepted append")?;
    assert!(error.to_string().contains("resource-limit"));
    assert_eq!(file.metadata()?.len(), MAX_ARTIFACT);
    Ok(())
}

#[test]
fn publication_rejects_oversize_body_before_creating_a_file() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("oversize.json");
    let length = usize::try_from(MAX_ARTIFACT)?
        .checked_add(1)
        .context("length overflow")?;
    let error = write_new(&path, &vec![b'x'; length])
        .err()
        .context("oversize body accepted")?;
    assert!(error.to_string().contains("resource-limit"));
    assert!(!path.exists());
    Ok(())
}

#[test]
fn json_escaping_is_bounded_before_publication() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("escaped.json");
    let value = json!("\n".repeat(usize::try_from(MAX_ARTIFACT / 2)?));
    let error = write_json(&path, &value)
        .err()
        .context("escaped oversize JSON accepted")?;
    assert!(format!("{error:#}").contains("resource-limit"));
    assert!(!path.exists());
    Ok(())
}

#[test]
fn concurrent_near_cap_writers_accept_only_one_complete_record() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("ledger.jsonl");
    let mut file = std::fs::File::create(&path)?;
    file.write_all(b"preserved-prefix")?;
    file.set_len(MAX_ARTIFACT.checked_sub(5).context("limit underflow")?)?;
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let writer = || {
            barrier.wait();
            append(&path, &Value::Null)
        };
        let first = scope.spawn(writer);
        let second = scope.spawn(writer);
        Ok::<_, anyhow::Error>([
            first
                .join()
                .map_err(|_| anyhow::anyhow!("first writer panicked"))?,
            second
                .join()
                .map_err(|_| anyhow::anyhow!("second writer panicked"))?,
        ])
    })?;
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(file.metadata()?.len(), MAX_ARTIFACT);
    let mut reader = std::fs::File::open(&path)?;
    let mut prefix = [0_u8; 16];
    reader.read_exact(&mut prefix)?;
    assert_eq!(&prefix, b"preserved-prefix");
    std::io::Seek::seek(&mut reader, std::io::SeekFrom::End(-5))?;
    let mut tail = [0_u8; 5];
    reader.read_exact(&mut tail)?;
    assert_eq!(&tail, b"null\n");
    Ok(())
}

#[test]
fn contending_writer_fails_within_budget_without_truncating() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("ledger.jsonl");
    write_new(&path, b"retained\n")?;
    let locked = std::fs::OpenOptions::new().append(true).open(&path)?;
    locked.try_lock()?;
    std::thread::scope(|scope| {
        scope
            .spawn(|| append(&path, &Value::Null))
            .join()
            .map_err(|_| anyhow::anyhow!("contending writer panicked"))
    })?
    .err()
    .context("contending append accepted")?;
    assert_eq!(std::fs::read(&path)?, b"retained\n");
    locked.unlock()?;
    append(&path, &Value::Null)?;
    assert_eq!(std::fs::read(&path)?, b"retained\nnull\n");
    Ok(())
}
