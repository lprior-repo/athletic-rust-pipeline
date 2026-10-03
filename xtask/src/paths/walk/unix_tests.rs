use super::tests::{write_source, SKIP};
use super::{run, run_with_limits, Limits, Selection};
use std::fs;
use std::os::unix::fs::symlink;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::process::Command;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn make_fifo(path: &Path) -> TestResult {
    let status = Command::new("mkfifo").arg("--").arg(path).status()?;
    check!(status.success(), "mkfifo failed: {status}");
    Ok(())
}

#[test]
fn root_symlinks_to_directories_and_files_are_refused() -> TestResult {
    let parent = tempfile::tempdir()?;
    write_source(parent.path(), "actual/safe.rs")?;
    symlink(
        parent.path().join("actual"),
        parent.path().join("directory-link"),
    )?;
    symlink(
        parent.path().join("actual/safe.rs"),
        parent.path().join("file-link"),
    )?;
    for link in [
        "directory-link",
        "directory-link/",
        "directory-link/.",
        "file-link",
    ] {
        for selection in [Selection::Rust, Selection::All] {
            let error = run(&parent.path().join(link), &[], selection)
                .err()
                .ok_or("accepted a linked root")?;
            check!(error.to_string().contains("real directory"), "{error}");
            check!(error.to_string().contains("link"), "{error}");
        }
    }
    Ok(())
}

#[test]
fn rust_root_ancestor_links_are_refused_without_changing_all_mode() -> TestResult {
    let parent = tempfile::tempdir()?;
    write_source(parent.path(), "actual/subdir/safe.rs")?;
    symlink(
        parent.path().join("actual"),
        parent.path().join("parent-link"),
    )?;
    let root = parent.path().join("parent-link/subdir");
    let error = run(&root, &[], Selection::Rust)
        .err()
        .ok_or("accepted a root with a linked ancestor")?;
    check!(error.to_string().contains("symlinks"), "{error}");
    check!(error.to_string().contains("parent-link"), "{error}");
    let paths = run(&root, &[], Selection::All)?;
    check!(eq; paths, vec![root.join("safe.rs")]);
    Ok(())
}

#[test]
fn parent_components_cannot_lexically_erase_a_root_ancestor_link() -> TestResult {
    let parent = tempfile::tempdir()?;
    fs::create_dir_all(parent.path().join("actual/nested"))?;
    write_source(parent.path(), "actual/safe_dir/safe.rs")?;
    symlink(
        parent.path().join("actual/nested"),
        parent.path().join("parent-link"),
    )?;
    let root = parent.path().join("parent-link/../safe_dir");
    let error = run(&root, &[], Selection::Rust)
        .err()
        .ok_or("collapsed a symlink before a parent component")?;
    check!(error.to_string().contains("symlinks"), "{error}");
    check!(error.to_string().contains("parent-link"), "{error}");
    let paths = run(&root, &[], Selection::All)?;
    check!(eq; paths, vec![root.join("safe.rs")]);
    Ok(())
}

#[test]
fn rust_file_links_are_refused_even_with_an_ordinary_safe_source() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "safe.rs")?;
    symlink(root.path().join("safe.rs"), root.path().join("linked.rs"))?;
    let error = run(root.path(), &[], Selection::Rust)
        .err()
        .ok_or("accepted a Rust file link")?;
    check!(error.to_string().contains("symlink"), "{error}");
    check!(error.to_string().contains("linked.rs"), "{error}");
    Ok(())
}

#[test]
fn non_rust_and_dangling_links_cannot_escape_rust_coverage_checks() -> TestResult {
    for (name, target) in [("linked.txt", "safe.rs"), ("dangling", "missing")] {
        let root = tempfile::tempdir()?;
        write_source(root.path(), "safe.rs")?;
        symlink(root.path().join(target), root.path().join(name))?;
        let error = run(root.path(), &[], Selection::Rust)
            .err()
            .ok_or("accepted an unselected symlink")?;
        check!(error.to_string().contains("symlink"), "{error}");
        check!(error.to_string().contains(name), "{error}");
    }
    Ok(())
}

#[test]
fn rust_directory_links_cannot_hide_an_owned_subtree() -> TestResult {
    let root = tempfile::tempdir()?;
    let target = tempfile::tempdir()?;
    write_source(root.path(), "safe.rs")?;
    write_source(target.path(), "hidden.rs")?;
    symlink(target.path(), root.path().join("generated"))?;
    let error = run(root.path(), &[], Selection::Rust)
        .err()
        .ok_or("accepted a directory link")?;
    check!(error.to_string().contains("symlink"), "{error}");
    check!(error.to_string().contains("generated"), "{error}");
    Ok(())
}

#[test]
fn directory_and_self_link_cycles_are_refused_without_following() -> TestResult {
    for cycle in ["back", "self"] {
        let root = tempfile::tempdir()?;
        write_source(root.path(), "safe.rs")?;
        fs::create_dir(root.path().join("nested"))?;
        let link = root.path().join("nested").join(cycle);
        let target = if cycle == "back" {
            root.path()
        } else {
            link.as_path()
        };
        symlink(target, &link)?;
        let error = run(root.path(), &[], Selection::Rust)
            .err()
            .ok_or("accepted a symlink cycle")?;
        check!(error.to_string().contains("symlink"), "{error}");
        check!(error.to_string().contains(cycle), "{error}");
    }
    Ok(())
}

#[test]
fn all_six_excluded_names_are_applied_before_symlink_refusal() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "safe.rs")?;
    fs::create_dir(root.path().join("nested"))?;
    for name in SKIP {
        symlink(
            root.path().join("missing"),
            root.path().join("nested").join(name),
        )?;
    }
    let paths = run_with_limits(
        root.path(),
        SKIP,
        Selection::Rust,
        Limits {
            entries: 8,
            files: 1,
            depth: 2,
        },
    )?;
    check!(eq; paths, vec![root.path().join("safe.rs")]);
    let error = run_with_limits(
        root.path(),
        SKIP,
        Selection::Rust,
        Limits {
            entries: 7,
            files: 1,
            depth: 2,
        },
    )
    .err()
    .ok_or("excluded symlinks escaped entry admission")?;
    check!(error.to_string().contains("visited entry limit"), "{error}");
    Ok(())
}

#[test]
fn all_mode_keeps_leaf_links_without_traversing_their_targets() -> TestResult {
    let root = tempfile::tempdir()?;
    let target = tempfile::tempdir()?;
    write_source(root.path(), "safe.rs")?;
    write_source(target.path(), "hidden.rs")?;
    symlink(target.path(), root.path().join("directory-link"))?;
    symlink(root.path(), root.path().join("cycle"))?;
    symlink(
        root.path().join("safe.rs"),
        root.path().join("file-link.rs"),
    )?;
    symlink(root.path().join("missing"), root.path().join("vendor"))?;
    let paths = run_with_limits(
        root.path(),
        SKIP,
        Selection::All,
        Limits {
            entries: 5,
            files: 5,
            depth: 1,
        },
    )?;
    check!(eq; paths, vec![
        root.path().join("cycle"), root.path().join("directory-link"),
        root.path().join("file-link.rs"), root.path().join("safe.rs"),
        root.path().join("vendor"),
    ]);
    Ok(())
}

#[test]
fn selected_fifo_is_refused_without_opening_it_for_reading() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "safe.rs")?;
    make_fifo(&root.path().join("blocked.rs"))?;
    let error = run(root.path(), &[], Selection::Rust)
        .err()
        .ok_or("accepted a selected FIFO")?;
    check!(error.to_string().contains("not a regular file"), "{error}");
    check!(error.to_string().contains("blocked.rs"), "{error}");
    let paths = run(root.path(), &[], Selection::All)?;
    check!(eq; paths, vec![root.path().join("blocked.rs"), root.path().join("safe.rs")]);
    Ok(())
}

#[test]
fn non_rust_fifo_is_not_a_selected_source_but_remains_an_all_mode_leaf() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "safe.rs")?;
    make_fifo(&root.path().join("pipe.txt"))?;
    let paths = run(root.path(), &[], Selection::Rust)?;
    check!(eq; paths, vec![root.path().join("safe.rs")]);
    let paths = run(root.path(), &[], Selection::All)?;
    check!(eq; paths, vec![root.path().join("pipe.txt"), root.path().join("safe.rs")]);
    Ok(())
}

#[test]
fn selected_socket_is_refused_while_all_mode_preserves_leaf_semantics() -> TestResult {
    let root = tempfile::tempdir()?;
    write_source(root.path(), "safe.rs")?;
    let socket = root.path().join("socket.rs");
    let listener = UnixListener::bind(&socket)?;
    let error = run(root.path(), &[], Selection::Rust)
        .err()
        .ok_or("accepted a selected socket")?;
    check!(error.to_string().contains("not a regular file"), "{error}");
    check!(error.to_string().contains("socket.rs"), "{error}");
    let paths = run(root.path(), &[], Selection::All)?;
    check!(eq; paths, vec![root.path().join("safe.rs"), socket]);
    drop(listener);
    Ok(())
}
