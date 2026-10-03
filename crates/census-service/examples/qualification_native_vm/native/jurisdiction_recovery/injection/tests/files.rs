use super::super::{files as private_files, MARKER};
use super::{failure, fixture};
use anyhow::Result;
use std::os::unix::fs::{symlink, PermissionsExt};

#[test]
fn artifact_reader_accepts_exact_budget_and_refuses_one_byte_over() -> Result<()> {
    let directory = fixture::directory()?;
    let accepted = directory.path().join("accepted.json");
    fixture::write_private(&accepted, &vec![b'x'; 4096])?;
    assert_eq!(private_files::read_required(&accepted)?, vec![b'x'; 4096]);
    let refused = directory.path().join("refused.json");
    fixture::write_private(&refused, &vec![b'x'; 4097])?;
    assert!(failure(private_files::read_required(&refused))?.contains("exceeds 4096 bytes"));
    Ok(())
}

#[test]
fn artifact_reader_refuses_symlink_nonregular_public_and_multilink_files() -> Result<()> {
    let directory = fixture::directory()?;
    let target = directory.path().join("target.json");
    fixture::write_private(&target, b"private-original")?;
    let link = directory.path().join("symlink.json");
    symlink(&target, &link)?;
    assert!(
        failure(private_files::read_required(&link))?.contains("private regular single-link file")
    );
    let subdirectory = directory.path().join("directory.json");
    std::fs::create_dir(&subdirectory)?;
    assert!(failure(private_files::read_required(&subdirectory))?
        .contains("private regular single-link file"));
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644))?;
    assert!(failure(private_files::read_required(&target))?
        .contains("private regular single-link file"));
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))?;
    let hardlink = directory.path().join("hardlink.json");
    std::fs::hard_link(&target, &hardlink)?;
    assert!(failure(private_files::read_required(&target))?
        .contains("private regular single-link file"));
    assert_eq!(std::fs::read(&target)?, b"private-original".to_vec());
    Ok(())
}

#[test]
fn config_refuses_dangling_symlink_conflicts_without_following_or_replacing_them() -> Result<()> {
    let (original, _) = fixture::active()?;
    let directory = fixture::directory()?;
    let missing = directory.path().join("never-created.json");
    let config = directory.path().join("native-source-boundary-config.json");
    symlink(&missing, &config)?;
    assert!(failure(fixture::configured(directory.path(), &original))?
        .contains("reused or conflicting artifact"));
    assert_eq!(std::fs::read_link(&config)?, missing);
    assert!(!directory.path().join("never-created.json").exists());
    Ok(())
}

#[test]
fn artifact_reader_refuses_symlinked_parent_even_with_private_target_permissions() -> Result<()> {
    let directory = fixture::directory()?;
    let target = directory.path().join("real");
    std::fs::create_dir(&target)?;
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o700))?;
    let parent = directory.path().join("linked");
    symlink(&target, &parent)?;
    assert!(failure(private_files::read_required(&parent.join(MARKER)))?
        .contains("owned private nonsymlink directory"));
    Ok(())
}

#[test]
fn absent_marker_is_pending_but_absent_required_config_is_explicit_failure() -> Result<()> {
    let directory = fixture::directory()?;
    assert_eq!(
        private_files::published_marker(&directory.path().join(MARKER))?,
        None
    );
    assert!(failure(private_files::read_required(
        &directory.path().join("native-source-boundary-config.json")
    ))?
    .contains("native boundary artifact missing"));
    Ok(())
}

#[test]
fn unsafe_pending_marker_is_refused_instead_of_being_trusted_as_publication_progress() -> Result<()>
{
    let directory = fixture::directory()?;
    let target = directory.path().join("target.json");
    fixture::write_private(&target, b"private-original")?;
    let marker = directory.path().join(MARKER);
    let pending = private_files::pending(&marker)?;
    symlink(&target, &pending)?;
    assert!(
        failure(private_files::published_marker(&marker))?.contains("pending artifact is unsafe")
    );
    assert_eq!(std::fs::read_link(&pending)?, target);
    Ok(())
}
