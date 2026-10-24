use super::*;

#[cfg(unix)]
#[test]
fn checked_recovery_refuses_symlinked_object_and_preserves_link_and_target() -> TestResult {
    let fixture = Fixture::new()?;
    let reference = fixture.install(b"linked object bytes")?;
    let path = fixture.directory.join(&reference.digest);
    let target = fixture.root.path().join("link-target");
    fs::rename(&path, &target)?;
    std::os::unix::fs::symlink(&target, &path)?;
    invariant(
        InitialObjects::checked_recovery(&fixture.directory),
        "selected proof inventory contains an invalid regular object",
    )?;
    check!(fs::symlink_metadata(&path)?.file_type().is_symlink());
    check!(eq; fs::read_link(&path)?, target);
    check!(eq; fs::read(&target)?, b"linked object bytes");
    fixture.sentinel()
}

#[cfg(unix)]
#[test]
fn initial_policies_refuse_symlinked_proof_directory_without_following_or_deleting_it() -> TestResult
{
    let fixture = Fixture::new()?;
    let reference = fixture.install(b"linked directory proof")?;
    let alias = fixture.root.path().join("proof-alias");
    std::os::unix::fs::symlink(&fixture.directory, &alias)?;
    invariant(
        InitialObjects::checked_recovery(&alias),
        "selected proof directory or ancestor is not a regular directory",
    )?;
    invariant(
        InitialObjects::fresh().require_initial(&alias),
        "selected proof directory or ancestor is not a regular directory",
    )?;
    check!(eq; fs::read_link(&alias)?, fixture.directory);
    fixture.object(&reference, b"linked directory proof")?;
    fixture.sentinel()
}

#[cfg(unix)]
#[test]
fn initial_policies_refuse_symlinked_ancestor_even_when_leaf_is_a_real_directory() -> TestResult {
    let fixture = Fixture::new()?;
    let real = fixture.root.path().join("real-parent");
    fs::create_dir(&real)?;
    let nested = real.join("nested-proofs");
    fs::create_dir(&nested)?;
    let bytes = b"ancestor target proof";
    let digest = proof_objects::install(&nested, bytes)?;
    let alias = fixture.root.path().join("parent-alias");
    std::os::unix::fs::symlink(&real, &alias)?;
    let through_alias = alias.join("nested-proofs");
    check!(fs::symlink_metadata(&through_alias)?.file_type().is_dir());
    invariant(
        InitialObjects::checked_recovery(&through_alias),
        "selected proof directory or ancestor is not a regular directory",
    )?;
    invariant(
        InitialObjects::fresh().require_initial(&through_alias),
        "selected proof directory or ancestor is not a regular directory",
    )?;
    check!(eq; fs::read_link(&alias)?, real);
    check!(eq; fs::read(nested.join(&digest))?, bytes);
    fixture.sentinel()
}
