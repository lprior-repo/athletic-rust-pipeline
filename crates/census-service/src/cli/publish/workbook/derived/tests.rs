use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn derived_sibling_accepts_fresh_empty_and_retry_layout_without_creating_output() -> TestResult {
    let directory = tempfile::tempdir()?;
    let original = directory.path().join("original");
    let derived = directory.path().join("derived");
    std::fs::create_dir(&original)?;
    check!(eq; destination(&original, &derived)?, derived);
    check!(!derived.exists());
    std::fs::create_dir(&derived)?;
    check!(eq; destination(&original, &derived)?, derived);
    std::fs::create_dir(derived.join("generations"))?;
    check!(eq; destination(&original, &derived)?, derived);
    Ok(())
}

#[test]
fn derived_root_refuses_original_relatives_files_and_nonpublication_data() -> TestResult {
    let directory = tempfile::tempdir()?;
    let original = directory.path().join("original");
    std::fs::create_dir(&original)?;
    for requested in [
        original.clone(),
        directory.path().to_path_buf(),
        original.join("nested"),
        original.join(".."),
    ] {
        check!(destination(&original, &requested).is_err());
    }
    let unrelated = directory.path().join("data");
    std::fs::create_dir(&unrelated)?;
    std::fs::write(unrelated.join("retained-input"), b"preserve")?;
    check!(destination(&original, &unrelated).is_err());
    let file = directory.path().join("file");
    std::fs::write(&file, b"preserve")?;
    check!(destination(&original, &file).is_err());
    check!(eq; std::fs::read(&file)?, b"preserve");
    check!(eq; std::fs::read(unrelated.join("retained-input"))?, b"preserve");
    Ok(())
}

#[cfg(unix)]
#[test]
fn derived_root_refuses_symlink_roots_and_generation_layouts() -> TestResult {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir()?;
    let original = directory.path().join("original");
    let alias = directory.path().join("alias");
    let derived = directory.path().join("derived");
    std::fs::create_dir(&original)?;
    symlink(&original, &alias)?;
    check!(destination(&original, &alias).is_err());
    check!(destination(&alias, &derived).is_err());
    std::fs::create_dir(&derived)?;
    symlink(&original, derived.join("generations"))?;
    check!(destination(&original, &derived).is_err());
    let dangling = directory.path().join("dangling");
    symlink(directory.path().join("absent"), &dangling)?;
    check!(destination(&original, &dangling).is_err());
    let parent_alias = directory.path().join("parent-alias");
    symlink(directory.path(), &parent_alias)?;
    check!(destination(&original, &parent_alias.join("original")).is_err());
    check!(eq; destination(&original, &parent_alias.join("fresh"))?, directory.path().join("fresh"));
    Ok(())
}
