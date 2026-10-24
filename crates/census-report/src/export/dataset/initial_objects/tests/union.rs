use super::*;

#[test]
fn checked_recovery_unions_new_roots_installed_after_admission_in_either_order() -> TestResult {
    let fixture = Fixture::new()?;
    let initial = fixture.install(b"initial retained proof")?;
    let policy = InitialObjects::checked_recovery(&fixture.directory)?;
    let first = fixture.install(b"new first reported root")?;
    let second = fixture.install(b"new second reported root")?;
    invariant(
        policy.require_initial(&fixture.directory),
        "selected proof directory differs from its authenticated object inventory",
    )?;
    policy.require_with_roots(&fixture.directory, &[first.clone(), second.clone()])?;
    policy.require_with_roots(&fixture.directory, &[second.clone(), first.clone()])?;
    fixture.object(&initial, b"initial retained proof")?;
    fixture.object(&first, b"new first reported root")?;
    fixture.object(&second, b"new second reported root")?;
    fixture.sentinel()
}

#[test]
fn checked_recovery_deduplicates_full_identical_roots_within_and_across_initial() -> TestResult {
    let fixture = Fixture::new()?;
    let initial = fixture.install(b"initial duplicate proof")?;
    let policy = InitialObjects::checked_recovery(&fixture.directory)?;
    let added = fixture.install(b"new duplicate proof")?;
    policy.require_with_roots(
        &fixture.directory,
        &[
            added.clone(),
            initial.clone(),
            added.clone(),
            initial.clone(),
        ],
    )?;
    check!(eq; objects::inventory(&fixture.directory)?.len(), 2);
    fixture.object(&initial, b"initial duplicate proof")?;
    fixture.object(&added, b"new duplicate proof")?;
    fixture.sentinel()
}

#[test]
fn checked_recovery_refuses_same_digest_with_conflicting_full_reference_length() -> TestResult {
    let fixture = Fixture::new()?;
    let initial = fixture.install(b"retained proof")?;
    let policy = InitialObjects::checked_recovery(&fixture.directory)?;
    let mut conflict = initial.clone();
    conflict.bytes = conflict.bytes.checked_add(1).ok_or("test byte overflow")?;
    invariant(
        policy.require_with_roots(&fixture.directory, &[conflict]),
        "selected proof inventory is noncanonical or exceeds an object limit",
    )?;
    fixture.object(&initial, b"retained proof")?;
    policy.require_initial(&fixture.directory)?;
    fixture.sentinel()
}

#[test]
fn checked_recovery_refuses_missing_new_root_without_adopting_a_foreign_directory() -> TestResult {
    let fixture = Fixture::new()?;
    let foreign = Fixture::new()?;
    let initial = fixture.install(b"initial existing proof")?;
    let missing = foreign.install(b"new expected but absent here")?;
    let policy = InitialObjects::checked_recovery(&fixture.directory)?;
    invariant(
        policy.require_with_roots(&fixture.directory, std::slice::from_ref(&missing)),
        "selected proof directory differs from its authenticated object inventory",
    )?;
    check!(!fixture.directory.join(&missing.digest).exists());
    fixture.object(&initial, b"initial existing proof")?;
    foreign.object(&missing, b"new expected but absent here")?;
    fixture.sentinel()?;
    foreign.sentinel()
}

#[test]
fn checked_recovery_refuses_unreported_extra_created_after_initial_admission() -> TestResult {
    let fixture = Fixture::new()?;
    let initial = fixture.install(b"initial existing proof")?;
    let policy = InitialObjects::checked_recovery(&fixture.directory)?;
    let expected = fixture.install(b"new expected root")?;
    let extra = fixture.install(b"new unexpected root")?;
    invariant(
        policy.require_with_roots(&fixture.directory, std::slice::from_ref(&expected)),
        "selected proof directory differs from its authenticated object inventory",
    )?;
    fixture.object(&initial, b"initial existing proof")?;
    fixture.object(&expected, b"new expected root")?;
    fixture.object(&extra, b"new unexpected root")?;
    fixture.sentinel()
}

#[test]
fn fresh_accepts_exact_new_roots_and_rejects_any_unreported_existing_proof() -> TestResult {
    let fixture = Fixture::new()?;
    let policy = InitialObjects::fresh();
    let first = fixture.install(b"first new root")?;
    let second = fixture.install(b"second new root")?;
    invariant(
        policy.require_with_roots(&fixture.directory, std::slice::from_ref(&first)),
        "selected proof directory differs from its authenticated object inventory",
    )?;
    policy.require_with_roots(
        &fixture.directory,
        &[second.clone(), first.clone(), second.clone()],
    )?;
    fixture.object(&first, b"first new root")?;
    fixture.object(&second, b"second new root")?;
    fixture.sentinel()
}

#[test]
fn empty_checked_recovery_accepts_exact_new_roots_installed_after_admission() -> TestResult {
    let fixture = Fixture::new()?;
    let policy = InitialObjects::checked_recovery(&fixture.directory)?;
    let root = fixture.install(b"new root after empty admission")?;
    invariant(
        policy.require_initial(&fixture.directory),
        "selected proof capture requires an empty owned directory",
    )?;
    policy.require_with_roots(&fixture.directory, &[root.clone(), root.clone()])?;
    fixture.object(&root, b"new root after empty admission")?;
    fixture.sentinel()
}

#[test]
fn new_roots_with_same_digest_and_conflicting_lengths_refuse_without_mutation() -> TestResult {
    let fixture = Fixture::new()?;
    let policy = InitialObjects::checked_recovery(&fixture.directory)?;
    let root = fixture.install(b"new conflicting root")?;
    let mut conflict = root.clone();
    conflict.bytes = conflict.bytes.checked_add(1).ok_or("test byte overflow")?;
    invariant(
        policy.require_with_roots(&fixture.directory, &[root.clone(), conflict]),
        "selected proof inventory is noncanonical or exceeds an object limit",
    )?;
    fixture.object(&root, b"new conflicting root")?;
    fixture.sentinel()
}

#[test]
fn new_root_checksum_is_verified_after_initial_policy_admission() -> TestResult {
    let fixture = Fixture::new()?;
    let initial = fixture.install(b"initial sound proof")?;
    let policy = InitialObjects::checked_recovery(&fixture.directory)?;
    let original = b"new reported root bytes";
    let root = fixture.install(original)?;
    let changed = vec![b'x'; original.len()];
    check!(eq; u64::try_from(changed.len())?, root.bytes);
    check!(ne; changed.as_slice(), original);
    let path = fixture.directory.join(&root.digest);
    fs::write(&path, &changed)?;
    corrupt(
        policy.require_with_roots(&fixture.directory, std::slice::from_ref(&root)),
        &path,
    )?;
    fixture.object(&initial, b"initial sound proof")?;
    fixture.object(&root, &changed)?;
    fixture.sentinel()
}
