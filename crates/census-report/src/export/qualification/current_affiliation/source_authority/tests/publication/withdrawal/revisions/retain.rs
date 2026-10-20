use tempfile::TempDir;

pub(super) fn maybe_retain(
    archive: TempDir,
    outputs: [TempDir; 3],
    revisions: [&super::frozen::FrozenRevision; 3],
) -> super::TestResult {
    if std::env::var_os("CENSUS_CAPTURE_REVISIONS_RETAIN").is_none() {
        return Ok(());
    }
    let [output_g0, output_g1, output_g2] = outputs;
    let [g0, g1, g2] = revisions;
    let receipt = serde_json::json!({
        "archive": archive.path(),
        "output_g0": output_g0.path(),
        "output_g1": output_g1.path(),
        "output_g2": output_g2.path(),
        "frozen_g0": { "path": g0.path, "digest": g0.digest },
        "frozen_g1": { "path": g1.path, "digest": g1.digest },
        "frozen_g2": { "path": g2.path, "digest": g2.digest },
        "workbook_g0": { "path": g0.workbook, "generation_digest": g0.generation_digest },
        "workbook_g1": { "path": g1.workbook, "generation_digest": g1.generation_digest },
        "workbook_g2": { "path": g2.workbook, "generation_digest": g2.generation_digest },
        "lineage_g0": g0.lineage,
        "lineage_g1": g1.lineage,
        "lineage_g2": g2.lineage,
    });
    let encoded = serde_json::to_string(&receipt)?;
    let _retained = [
        archive.keep(),
        output_g0.keep(),
        output_g1.keep(),
        output_g2.keep(),
    ];
    eprintln!("CENSUS_CAPTURE_REVISIONS_RETAINED={encoded}");
    Ok(())
}
