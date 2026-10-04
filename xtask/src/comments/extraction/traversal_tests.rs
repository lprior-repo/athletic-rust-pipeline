use super::run;
use std::fs;
use std::path::Path;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn write_source(root: &Path, relative: &str, source: &str) -> TestResult {
    let path = root.join(relative);
    let parent = path.parent().ok_or("fixture has no parent")?;
    fs::create_dir_all(parent)?;
    fs::write(path, source)?;
    Ok(())
}

#[test]
fn empty_roots_cannot_succeed_using_rendered_templates_alone() -> TestResult {
    let root = tempfile::tempdir()?;
    let error = run(root.path())
        .err()
        .ok_or("accepted an empty source graph")?;
    check!(eq; error.to_string(), "no project-owned Rust source files found");
    Ok(())
}

#[test]
fn excluded_sources_and_non_rust_evidence_do_not_count_as_examined_sources() -> TestResult {
    let root = tempfile::tempdir()?;
    for directory in [".git", ".jj", "target", "var", "vendor", "node_modules"] {
        write_source(
            root.path(),
            &format!("{directory}/nested/bypass.rs"),
            "value.unwrap();",
        )?;
    }
    write_source(
        root.path(),
        "tests/fixtures/evidence.txt",
        "value.unwrap();",
    )?;
    let error = run(root.path())
        .err()
        .ok_or("accepted an excluded-only source graph")?;
    check!(eq; error.to_string(), "no project-owned Rust source files found");
    Ok(())
}

#[test]
fn actual_fixture_graph_includes_tests_examples_benches_and_tools() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "src/lib.rs", "fn safe() {}")?;
    for relative in [
        "tests/integration.rs",
        "tests/fixtures/nested/input.rs",
        "examples/example.rs",
        "benches/bench.rs",
        "tools/nested/helper.rs",
    ] {
        write_source(
            root.path(),
            relative,
            "#[cfg(any())] fn hidden() { value.unwrap(); }",
        )?;
    }
    let error = run(root.path())
        .err()
        .ok_or("accepted extractions in the fixture graph")?;
    check!(eq; error.to_string(), "panic-extraction policy: 6 Rust files and 0 rendered templates contain violations");
    Ok(())
}

#[test]
fn clean_project_sources_ignore_excluded_sources_and_quoted_input_fixtures() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(
        root.path(),
        "src/lib.rs",
        "fn safe() { let _ = value.map_or(0, core::convert::identity); }",
    )?;
    write_source(
        root.path(),
        "tests/fixtures/quoted.rs",
        "const INPUT: &str = r##\"value.unwrap(); #[allow(clippy::all)]\"##;",
    )?;
    for directory in [".git", ".jj", "target", "var", "vendor", "node_modules"] {
        write_source(
            root.path(),
            &format!("crates/nested/{directory}/bad.rs"),
            "value.expect();",
        )?;
    }
    run(root.path())?;
    Ok(())
}

#[test]
fn a_later_owned_file_cannot_hide_behind_a_clean_earlier_file() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "a.rs", "fn safe() {}")?;
    write_source(
        root.path(),
        "z.rs",
        "#[cfg_attr(test, expect(clippy::expect_used))] fn f() {}",
    )?;
    let error = run(root.path())
        .err()
        .ok_or("accepted later lint suppression")?;
    check!(eq; error.to_string(), "panic-extraction policy: 1 Rust files and 0 rendered templates contain violations");
    Ok(())
}

#[test]
fn unterminated_owned_source_reports_path_and_literal_line() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(
        root.path(),
        "tests/fixtures/refusal.rs",
        "fn safe() {}\nconst INPUT: &str = \"unfinished",
    )?;
    let error = run(root.path())
        .err()
        .ok_or("accepted an unterminated fixture")?;
    let chain = format!("{error:#}");
    check!(
        chain.contains("tests/fixtures/refusal.rs: source refused"),
        "{chain}"
    );
    check!(
        chain.contains("line 2: unterminated Rust literal"),
        "{chain}"
    );
    Ok(())
}

#[test]
fn oversized_owned_sources_are_refused_before_reading() -> TestResult {
    let root = tempfile::tempdir()?;
    let path = root.path().join("oversized.rs");
    fs::File::create(&path)?.set_len(
        crate::comments::MAX_SOURCE_BYTES
            .checked_add(1)
            .ok_or("size overflow")?,
    )?;
    let error = run(root.path())
        .err()
        .ok_or("accepted an oversized source")?;
    check!(
        error.to_string().contains("source exceeds byte limit"),
        "{error}"
    );
    check!(error.to_string().contains("oversized.rs"), "{error}");
    Ok(())
}
