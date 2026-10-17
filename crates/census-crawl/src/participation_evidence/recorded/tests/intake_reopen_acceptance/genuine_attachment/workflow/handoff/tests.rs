use super::*;

#[test]
fn documentary_handoff_hashes_sorted_complete_regular_files() -> TestResult {
    let directory = tempfile::tempdir()?;
    std::fs::create_dir(directory.path().join("nested"))?;
    std::fs::write(directory.path().join("z"), b"abc")?;
    std::fs::write(directory.path().join("nested/a"), b"")?;
    let found = serde_json::to_value(inventory::read(directory.path())?)?;
    check!(eq; found, json!([
        {"path":"nested/a","bytes":0,"sha256":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"},
        {"path":"z","bytes":3,"sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}
    ]));
    Ok(())
}

#[test]
fn documentary_handoff_rejects_symlink_entries_and_roots() -> TestResult {
    let directory = tempfile::tempdir()?;
    let target = tempfile::tempdir()?;
    std::os::unix::fs::symlink(target.path(), directory.path().join("linked"))?;
    check!(inventory::read(directory.path()).is_err());
    check!(inventory::read(&directory.path().join("linked")).is_err());
    Ok(())
}

#[test]
fn documentary_handoff_qualifier_rejects_nonregular_file_types() -> TestResult {
    use rustix::fs::{FileType, OFlags};
    for kind in [
        FileType::Fifo,
        FileType::Socket,
        FileType::CharacterDevice,
        FileType::BlockDevice,
        FileType::Symlink,
    ] {
        check!(inventory::entry_flags(kind).is_err());
    }
    check!(inventory::entry_flags(FileType::RegularFile)?.contains(OFlags::NOFOLLOW));
    check!(
        inventory::entry_flags(FileType::Directory)?.contains(OFlags::DIRECTORY | OFlags::NOFOLLOW)
    );
    Ok(())
}

#[test]
fn documentary_handoff_refuses_oversize_before_reading_file_bytes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let file = File::create(directory.path().join("large"))?;
    file.set_len(268_435_457)?;
    check!(inventory::read(directory.path()).is_err());
    Ok(())
}

#[test]
fn documentary_handoff_path_grammar_rejects_traversal_and_existing_targets() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    check!(eq; paths::destination(&root.join("new"))?, root.join("new"));
    check!(paths::destination(&root).is_err());
    check!(paths::destination(Path::new("relative/new")).is_err());
    check!(paths::destination(&root.join("../new")).is_err());
    for name in ["", ".", "..", "a/b", "a\\b", "a\nb"] {
        check!(paths::relative(Path::new("parent"), name).is_err());
    }
    check!(eq; paths::relative(Path::new("parent"), "child")?, Path::new("parent/child"));
    Ok(())
}

#[test]
fn documentary_handoff_requires_exact_lowercase_commit_and_tree_references() -> TestResult {
    validate_revision("0123456789abcdef0123456789abcdef01234567")?;
    for value in [
        "",
        "0123456789ABCDEF0123456789abcdef01234567",
        "0123456789abcdef0123456789abcdef0123456g",
        "0123456789abcdef0123456789abcdef012345678",
    ] {
        check!(validate_revision(value).is_err());
    }
    Ok(())
}

#[test]
fn documentary_handoff_manifest_is_new_and_cannot_overwrite() -> TestResult {
    let directory = tempfile::tempdir()?;
    let destination = directory.path().join("output");
    let original = json!({"sentinel":"unchanged"});
    write_manifest(&destination, &original)?;
    let bytes = std::fs::read(destination.join("handoff.json"))?;
    check!(eq; serde_json::from_slice::<Value>(&bytes)?, original);
    check!(write_manifest(&destination, &json!({"replacement":true})).is_err());
    check!(eq; std::fs::read(destination.join("handoff.json"))?, bytes);
    Ok(())
}
