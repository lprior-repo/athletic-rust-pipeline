use super::*;

#[test]
fn a_damaged_snapshot_fails_the_export_naming_its_line() {
    let dir = tempfile::tempdir().unwrap();
    let store_out = dir.path().join("out");
    std::fs::create_dir_all(&store_out).unwrap();
    let head = serde_json::json!({"id": "sch_0000000000000001", "name": "Ada Fixture High"});
    let tail = serde_json::json!({"id": "sch_0000000000000003", "name": "Bo Fixture High"});
    std::fs::write(
        store_out.join("schools.jsonl"),
        format!("{head}\n{{\"id\": \"sch_0000000000000002\"\n{tail}\n"),
    )
    .unwrap();
    let data = dir.path().join("data");

    let error = run_export_data(&ExportDataArgs {
        store_out,
        data: data.clone(),
    })
    .expect_err("a damaged snapshot must fail the export");

    let text = format!("{error:#}");
    assert!(
        text.contains("schools.jsonl"),
        "the error names the snapshot: {text}"
    );
    assert!(text.contains("line 2"), "the error names the line: {text}");
    assert!(
        !data.join("canonical-schools.csv").exists(),
        "a damaged snapshot publishes no CSV"
    );
}
