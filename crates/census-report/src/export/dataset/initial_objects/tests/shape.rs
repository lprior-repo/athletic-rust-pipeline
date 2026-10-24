use super::*;

#[test]
fn checked_recovery_refuses_malformed_and_temporary_names_without_deleting_them() -> TestResult {
    for name in ["not-a-sha256", ".staging.object.unfinished"] {
        let fixture = Fixture::new()?;
        let path = fixture.directory.join(name);
        fs::write(&path, b"preserved invalid object")?;
        invariant(
            InitialObjects::checked_recovery(&fixture.directory),
            "selected proof inventory contains an invalid regular object",
        )?;
        check!(eq; fs::read(&path)?, b"preserved invalid object");
        fixture.sentinel()?;
    }
    Ok(())
}

#[test]
fn checked_recovery_refuses_full_uppercase_digest_without_rewriting_it() -> TestResult {
    let fixture = Fixture::new()?;
    let reference = fixture.install(b"uppercase filename proof")?;
    let uppercase = reference.digest.to_ascii_uppercase();
    check!(eq; uppercase.len(), 64);
    check!(ne; uppercase, reference.digest);
    let lower = fixture.directory.join(&reference.digest);
    let path = fixture.directory.join(&uppercase);
    fs::rename(&lower, &path)?;
    invariant(
        InitialObjects::checked_recovery(&fixture.directory),
        "selected proof inventory contains an invalid regular object",
    )?;
    check!(!lower.exists());
    check!(eq; fs::read(&path)?, b"uppercase filename proof");
    fixture.sentinel()
}

#[test]
fn checked_recovery_refuses_zero_length_object_under_genuine_digest_and_keeps_it() -> TestResult {
    let fixture = Fixture::new()?;
    let reference = fixture.install(b"nonempty original proof")?;
    let path = fixture.directory.join(&reference.digest);
    fs::write(&path, b"")?;
    invariant(
        InitialObjects::checked_recovery(&fixture.directory),
        "selected proof inventory contains an invalid regular object",
    )?;
    fixture.object(&reference, b"")?;
    fixture.sentinel()
}

#[test]
fn checked_recovery_refuses_nested_directory_without_deleting_nested_content() -> TestResult {
    let fixture = Fixture::new()?;
    let nested = fixture.directory.join("nested");
    fs::create_dir(&nested)?;
    let sentinel = nested.join("preserved");
    fs::write(&sentinel, b"nested retained bytes")?;
    invariant(
        InitialObjects::checked_recovery(&fixture.directory),
        "selected proof inventory contains an invalid regular object",
    )?;
    check!(fs::symlink_metadata(&nested)?.file_type().is_dir());
    check!(eq; fs::read(&sentinel)?, b"nested retained bytes");
    fixture.sentinel()
}

#[test]
fn initial_policies_refuse_parent_escape_even_when_it_resolves_to_owned_proofs() -> TestResult {
    let fixture = Fixture::new()?;
    let reference = fixture.install(b"confined retained proof")?;
    let escaped = fixture.directory.join("..").join("proofs");
    check!(eq; fs::canonicalize(&escaped)?, fs::canonicalize(&fixture.directory)?);
    invariant(
        InitialObjects::checked_recovery(&escaped),
        "selected proof directory must have an explicit confined path",
    )?;
    invariant(
        InitialObjects::fresh().require_initial(&escaped),
        "selected proof directory must have an explicit confined path",
    )?;
    fixture.object(&reference, b"confined retained proof")?;
    fixture.sentinel()
}

#[test]
fn initial_policies_refuse_empty_path_before_any_ambient_directory_read() -> TestResult {
    let fixture = Fixture::new()?;
    for result in [
        InitialObjects::fresh().require_initial(Path::new("")),
        InitialObjects::checked_recovery(Path::new("")).map(|_| ()),
    ] {
        invariant(
            result,
            "selected proof directory must have an explicit confined path",
        )?;
    }
    fixture.sentinel()
}
