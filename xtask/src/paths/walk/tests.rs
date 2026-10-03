use super::{run, run_with_limits, Limits, Selection};
use std::fs;
use std::path::Path;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

pub(super) const SKIP: &[&str] = &[".git", ".jj", "target", "var", "vendor", "node_modules"];

pub(super) fn write_source(root: &Path, relative: &str) -> TestResult {
    let path = root.join(relative);
    let parent = path.parent().ok_or("fixture has no parent")?;
    fs::create_dir_all(parent)?;
    fs::write(path, "fn safe() {}")?;
    Ok(())
}

#[test]
fn empty_directory_needs_no_entry_or_file_admission() -> TestResult {
    let root = tempfile::tempdir()?;
    for selection in [Selection::All, Selection::Rust] {
        let paths = run_with_limits(
            root.path(),
            &[],
            selection,
            Limits {
                entries: 0,
                files: 0,
                depth: 1,
            },
        )?;
        check!(eq; paths, Vec::<std::path::PathBuf>::new());
    }
    Ok(())
}

#[test]
fn ordinary_file_is_never_accepted_as_the_root() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "safe.rs")?;
    for selection in [Selection::All, Selection::Rust] {
        let error = run(&root.path().join("safe.rs"), &[], selection)
            .err()
            .ok_or("accepted a file root")?;
        check!(error.to_string().contains("real directory"), "{error}");
        check!(error.to_string().contains("safe.rs"), "{error}");
    }
    Ok(())
}

#[test]
fn missing_root_reports_the_filesystem_failure() -> TestResult {
    let root = tempfile::tempdir()?;
    let error = run(&root.path().join("missing"), &[], Selection::Rust)
        .err()
        .ok_or("accepted a missing root")?;
    check!(error.to_string().contains("missing"), "{error}");
    check!(
        error.downcast_ref::<std::io::Error>().is_some(),
        "{error:#}"
    );
    Ok(())
}

#[test]
fn ordinary_parent_components_retain_filesystem_path_semantics() -> TestResult {
    let parent = tempfile::tempdir()?;
    fs::create_dir_all(parent.path().join("actual/nested"))?;
    write_source(parent.path(), "actual/safe_dir/safe.rs")?;
    let root = parent.path().join("actual/nested/../safe_dir");
    let paths = run(&root, &[], Selection::Rust)?;
    check!(eq; paths, vec![root.join("safe.rs")]);
    Ok(())
}

#[test]
fn rust_root_component_budget_admits_128_and_refuses_129() -> TestResult {
    let parent = tempfile::tempdir()?;
    let mut root = parent.path().to_path_buf();
    let initial = root.components().count();
    check!(
        initial < 128,
        "temporary root exceeds fixture component budget"
    );
    for _ in initial..128 {
        root.push("d");
        fs::create_dir(&root)?;
    }
    write_source(&root, "safe.rs")?;
    let paths = run(&root, &[], Selection::Rust)?;
    check!(eq; paths, vec![root.join("safe.rs")]);
    root.push("extra");
    fs::create_dir(&root)?;
    write_source(&root, "safe.rs")?;
    let error = run(&root, &[], Selection::Rust)
        .err()
        .ok_or("accepted more than 128 root components")?;
    check!(
        error.to_string().contains("root component limit"),
        "{error}"
    );
    let paths = run(&root, &[], Selection::All)?;
    check!(eq; paths, vec![root.join("safe.rs")]);
    Ok(())
}

#[test]
fn nested_rust_selection_is_sorted_and_exclusions_are_name_exact() -> TestResult {
    let root = tempfile::tempdir()?;
    for relative in [
        "z.rs",
        "src/b.rs",
        "src/a.rs",
        "tests/fixture.rs",
        "vendorish/kept.rs",
    ] {
        write_source(root.path(), relative)?;
    }
    write_source(root.path(), "notes.txt")?;
    write_source(root.path(), "upper.RS")?;
    for directory in SKIP {
        write_source(root.path(), &format!("src/{directory}/ignored.rs"))?;
    }
    let paths = run(root.path(), SKIP, Selection::Rust)?;
    check!(eq; paths, vec![
        root.path().join("src/a.rs"), root.path().join("src/b.rs"),
        root.path().join("tests/fixture.rs"), root.path().join("vendorish/kept.rs"),
        root.path().join("z.rs"),
    ]);
    Ok(())
}

#[test]
fn all_selection_keeps_non_rust_leaves_and_skips_only_directories() -> TestResult {
    let root = tempfile::tempdir()?;
    for relative in ["safe.rs", "notes.txt", "src/data.bin", "vendor"] {
        write_source(root.path(), relative)?;
    }
    write_source(root.path(), ".git/ignored.rs")?;
    let paths = run(root.path(), SKIP, Selection::All)?;
    check!(eq; paths, vec![
        root.path().join("notes.txt"), root.path().join("safe.rs"),
        root.path().join("src/data.bin"), root.path().join("vendor"),
    ]);
    Ok(())
}

#[test]
fn non_rust_leaves_consume_the_entry_budget_not_the_source_budget() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "safe.rs")?;
    write_source(root.path(), "data.txt")?;
    let limits = Limits {
        entries: 2,
        files: 1,
        depth: 1,
    };
    let paths = run_with_limits(root.path(), &[], Selection::Rust, limits)?;
    check!(eq; paths, vec![root.path().join("safe.rs")]);
    let error = run_with_limits(
        root.path(),
        &[],
        Selection::Rust,
        Limits {
            entries: 1,
            ..limits
        },
    )
    .err()
    .ok_or("accepted entries beyond their budget")?;
    check!(error.to_string().contains("visited entry limit"), "{error}");
    Ok(())
}

#[test]
fn every_yielded_directory_including_excluded_names_consumes_an_entry() -> TestResult {
    let root = tempfile::tempdir()?;
    fs::create_dir(root.path().join("empty"))?;
    write_source(root.path(), "vendor/deep/ignored.rs")?;
    let limits = Limits {
        entries: 2,
        files: 0,
        depth: 2,
    };
    let paths = run_with_limits(root.path(), SKIP, Selection::Rust, limits)?;
    check!(eq; paths, Vec::<std::path::PathBuf>::new());
    let error = run_with_limits(
        root.path(),
        SKIP,
        Selection::Rust,
        Limits {
            entries: 1,
            ..limits
        },
    )
    .err()
    .ok_or("excluded or empty directory escaped entry admission")?;
    check!(error.to_string().contains("visited entry limit"), "{error}");
    Ok(())
}

#[test]
fn first_yielded_entry_is_refused_when_the_entry_budget_is_zero() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "safe.rs")?;
    let error = run_with_limits(
        root.path(),
        &[],
        Selection::Rust,
        Limits {
            entries: 0,
            files: 1,
            depth: 1,
        },
    )
    .err()
    .ok_or("accepted an entry with no budget")?;
    check!(error.to_string().contains("visited entry limit"), "{error}");
    Ok(())
}

#[test]
fn selected_sources_are_admitted_at_the_exact_file_boundary() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "a.rs")?;
    write_source(root.path(), "b.rs")?;
    write_source(root.path(), "notes.txt")?;
    let limits = Limits {
        entries: 3,
        files: 2,
        depth: 1,
    };
    let paths = run_with_limits(root.path(), &[], Selection::Rust, limits)?;
    check!(eq; paths, vec![root.path().join("a.rs"), root.path().join("b.rs")]);
    let error = run_with_limits(
        root.path(),
        &[],
        Selection::Rust,
        Limits { files: 1, ..limits },
    )
    .err()
    .ok_or("accepted sources beyond their budget")?;
    check!(error.to_string().contains("selected file limit"), "{error}");
    Ok(())
}

#[test]
fn all_leaves_share_the_selected_file_boundary() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "a.rs")?;
    write_source(root.path(), "b.txt")?;
    let limits = Limits {
        entries: 2,
        files: 2,
        depth: 1,
    };
    let paths = run_with_limits(root.path(), &[], Selection::All, limits)?;
    check!(eq; paths, vec![root.path().join("a.rs"), root.path().join("b.txt")]);
    let error = run_with_limits(
        root.path(),
        &[],
        Selection::All,
        Limits { files: 1, ..limits },
    )
    .err()
    .ok_or("accepted leaves beyond their budget")?;
    check!(error.to_string().contains("selected file limit"), "{error}");
    Ok(())
}

#[test]
fn zero_file_budget_refuses_the_first_selected_source() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "safe.rs")?;
    let error = run_with_limits(
        root.path(),
        &[],
        Selection::Rust,
        Limits {
            entries: 1,
            files: 0,
            depth: 1,
        },
    )
    .err()
    .ok_or("accepted a source with no file budget")?;
    check!(error.to_string().contains("selected file limit"), "{error}");
    Ok(())
}

#[test]
fn root_frame_is_included_in_the_depth_limit() -> TestResult {
    let root = tempfile::tempdir()?;
    let error = run_with_limits(
        root.path(),
        &[],
        Selection::Rust,
        Limits {
            entries: 0,
            files: 0,
            depth: 0,
        },
    )
    .err()
    .ok_or("opened the root without a directory-frame budget")?;
    check!(
        error.to_string().contains("directory depth limit"),
        "{error}"
    );
    Ok(())
}

#[test]
fn nested_frames_are_admitted_at_the_exact_depth_boundary() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "one/two/safe.rs")?;
    let limits = Limits {
        entries: 3,
        files: 1,
        depth: 3,
    };
    let paths = run_with_limits(root.path(), &[], Selection::Rust, limits)?;
    check!(eq; paths, vec![root.path().join("one/two/safe.rs")]);
    let error = run_with_limits(
        root.path(),
        &[],
        Selection::Rust,
        Limits { depth: 2, ..limits },
    )
    .err()
    .ok_or("accepted too many nested frames")?;
    check!(
        error.to_string().contains("directory depth limit"),
        "{error}"
    );
    check!(error.to_string().contains("two"), "{error}");
    Ok(())
}

#[test]
fn even_an_empty_child_directory_requires_another_frame() -> TestResult {
    let root = tempfile::tempdir()?;
    fs::create_dir(root.path().join("empty"))?;
    let error = run_with_limits(
        root.path(),
        &[],
        Selection::Rust,
        Limits {
            entries: 1,
            files: 0,
            depth: 1,
        },
    )
    .err()
    .ok_or("empty child escaped the frame limit")?;
    check!(
        error.to_string().contains("directory depth limit"),
        "{error}"
    );
    Ok(())
}

#[test]
fn sibling_directories_reuse_the_frame_budget() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "one/a.rs")?;
    write_source(root.path(), "two/b.rs")?;
    let paths = run_with_limits(
        root.path(),
        &[],
        Selection::Rust,
        Limits {
            entries: 4,
            files: 2,
            depth: 2,
        },
    )?;
    check!(eq; paths, vec![root.path().join("one/a.rs"), root.path().join("two/b.rs")]);
    Ok(())
}
