use super::*;

#[test]
fn a_missing_snapshot_fails_the_counts_rather_than_counting_zero(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;

    let error = match counted_seeds(dir.path()) {
        Err(error) => error,
        Ok(_) => return Err("missing snapshot counted successfully".into()),
    };

    let text = format!("{error:#}");
    if !text.contains("athletes.jsonl") {
        return Err(format!("the error names the snapshot that is missing: {text}").into());
    }
    Ok(())
}
