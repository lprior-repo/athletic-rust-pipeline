use super::*;

pub(super) fn verify() -> TestResult {
    let (fixture, slower, faster) = observations()?;
    let facts = Facts::new()?;
    let directory = tempfile::tempdir()?;
    let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
    let (dataset, _objects) = facts.bind(&store, &writer, &[slower.clone(), faster.clone()])?;
    let path = directory.path().join("frozen-input.json");
    let digest = dataset.archive_digest()?;
    dataset.save_frozen(&path)?;
    let original_bytes = std::fs::read(&path)?;
    let view = facts.view(
        &dataset,
        [
            (
                slower.performance.id.clone(),
                facts.assessment(&slower.performance, &[ParticipationContext::Scholastic])?,
            ),
            (
                faster.performance.id.clone(),
                facts.assessment(&faster.performance, &[ParticipationContext::Collegiate])?,
            ),
        ],
    );
    let expected = selected(&dataset, &view, None)?;
    let mut newer = faster.clone();
    newer.performance.id = Id::mint("perf", &["newer-participation-evidence"]);
    newer.performance.source_key = "newer-participation-evidence".into();
    let reopened =
        match ExportDataset::reopen_frozen_with_digest(&path, &digest) {
            Err(error) => {
                check!(error
                    .to_string()
                    .contains("frozen affiliation descriptor differs from its bound input"));
                None
            }
            Ok(reopened) => {
                check!(eq; reopened.performances, dataset.performances);
                check!(eq; reopened.lineage.input_generation, dataset.lineage.input_generation);
                match selected(&reopened, &view, None) {
                    Err(error) => check!(error.to_string().contains(
                        "selected publication requires retained participation verification",
                    )),
                    Ok(_) => return Err(
                        "synthetic participation views survived a frozen roundtrip without proofs"
                            .into(),
                    ),
                }
                Some(reopened)
            }
        };
    check!(eq; std::fs::read(&path)?, original_bytes);
    check!(eq; selected(&dataset, &view, None)?, expected);
    let baseline = reopened
        .as_ref()
        .map_or(&dataset.lineage.input_generation, |reopened| {
            &reopened.lineage.input_generation
        })
        .clone();
    let (fresh, _fresh_objects) =
        facts.bind(&store, &writer, &[slower.clone(), faster.clone(), newer])?;
    check!(fresh.lineage.input_generation != baseline);
    match selected(&fresh, &view, None) {
        Err(error) => check!(error.to_string().contains("different frozen input")),
        Ok(_) => return Err("old view admitted newly acquired evidence".into()),
    }
    Ok(())
}
