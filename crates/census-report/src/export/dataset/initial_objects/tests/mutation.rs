use super::*;

fn admitted() -> TestResult<(
    Fixture,
    InitialObjects,
    objects::Reference,
    objects::Reference,
)> {
    let fixture = Fixture::new()?;
    let changed = fixture.install(b"admitted original bytes")?;
    let retained = fixture.install(b"another retained proof")?;
    let policy = InitialObjects::checked_recovery(&fixture.directory)?;
    Ok((fixture, policy, changed, retained))
}

#[test]
fn checked_recovery_refuses_added_valid_object_after_admission_and_keeps_all_bytes() -> TestResult {
    let (fixture, policy, original, retained) = admitted()?;
    let added = fixture.install(b"added unreported proof")?;
    invariant(
        policy.require_initial(&fixture.directory),
        "selected proof directory differs from its authenticated object inventory",
    )?;
    fixture.object(&original, b"admitted original bytes")?;
    fixture.object(&retained, b"another retained proof")?;
    fixture.object(&added, b"added unreported proof")?;
    fixture.sentinel()
}

#[test]
fn checked_recovery_refuses_removed_object_after_admission_without_restoring_it() -> TestResult {
    let (fixture, policy, removed, retained) = admitted()?;
    let path = fixture.directory.join(&removed.digest);
    fs::remove_file(&path)?;
    invariant(
        policy.require_initial(&fixture.directory),
        "selected proof directory differs from its authenticated object inventory",
    )?;
    check!(!path.exists());
    fixture.object(&retained, b"another retained proof")?;
    fixture.sentinel()
}

#[test]
fn checked_recovery_refuses_renamed_valid_digest_after_admission_without_rewriting_bytes(
) -> TestResult {
    let (fixture, policy, original, retained) = admitted()?;
    let foreign = Fixture::new()?;
    let renamed = foreign.install(b"genuine other digest")?;
    check!(ne; original.digest, renamed.digest);
    let old = fixture.directory.join(&original.digest);
    let new = fixture.directory.join(&renamed.digest);
    fs::rename(&old, &new)?;
    invariant(
        policy.require_initial(&fixture.directory),
        "selected proof directory differs from its authenticated object inventory",
    )?;
    check!(!old.exists());
    check!(eq; fs::read(&new)?, b"admitted original bytes");
    fixture.object(&retained, b"another retained proof")?;
    foreign.object(&renamed, b"genuine other digest")?;
    fixture.sentinel()?;
    foreign.sentinel()
}

#[test]
fn checked_recovery_refuses_truncated_object_after_admission_without_repairing_it() -> TestResult {
    let (fixture, policy, original, retained) = admitted()?;
    let changed = b"short";
    check!(u64::try_from(changed.len())? < original.bytes);
    fs::write(fixture.directory.join(&original.digest), changed)?;
    invariant(
        policy.require_initial(&fixture.directory),
        "selected proof directory differs from its authenticated object inventory",
    )?;
    fixture.object(&original, changed)?;
    fixture.object(&retained, b"another retained proof")?;
    fixture.sentinel()
}

#[test]
fn checked_recovery_refuses_same_length_corruption_after_admission_and_keeps_corrupt_bytes(
) -> TestResult {
    let (fixture, policy, original, retained) = admitted()?;
    let changed = b"corrupt! original bytes";
    check!(eq; u64::try_from(changed.len())?, original.bytes);
    check!(ne; changed, b"admitted original bytes");
    let path = fixture.directory.join(&original.digest);
    fs::write(&path, changed)?;
    corrupt(policy.require_initial(&fixture.directory), &path)?;
    corrupt(policy.require_with_roots(&fixture.directory, &[]), &path)?;
    fixture.object(&original, changed)?;
    fixture.object(&retained, b"another retained proof")?;
    fixture.sentinel()
}
