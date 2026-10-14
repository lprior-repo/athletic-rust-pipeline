use super::{
    gib_bytes, write_group, write_membership, CgroupMemoryReader, MemoryEnvelopeError, Path,
    TestResult,
};

#[test]
fn declared_parent_accounts_for_siblings_instead_of_only_the_endpoint_leaf() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc = tempfile::tempdir()?;
    let membership = proc.path().join("cgroup");
    let parent = write_group(
        mount.path(),
        "run",
        b"700\n",
        b"900\n",
        b"1073741824\n",
        b"0\n",
    )?;
    write_group(
        mount.path(),
        "run/endpoint",
        b"100\n",
        b"200\n",
        b"max\n",
        b"max\n",
    )?;
    write_membership(&membership, "/run/endpoint")?;
    let reader = CgroupMemoryReader::with_declared_paths(
        gib_bytes(1)?,
        membership,
        mount.path().into(),
        Some(Path::new("/run")),
    )?;
    let sample = reader.sample()?;
    check!(eq; sample.scope, parent);
    check!(eq; sample.current_bytes, 700);
    check!(eq; sample.peak_bytes, 900);
    Ok(())
}

#[test]
fn a_sibling_group_cannot_be_claimed_as_the_endpoint_envelope() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc = tempfile::tempdir()?;
    let membership = proc.path().join("cgroup");
    write_group(
        mount.path(),
        "other",
        b"100\n",
        b"100\n",
        b"1073741824\n",
        b"0\n",
    )?;
    write_membership(&membership, "/endpoint")?;
    let result = CgroupMemoryReader::with_declared_paths(
        gib_bytes(1)?,
        membership,
        mount.path().into(),
        Some(Path::new("/other")),
    );
    check!(matches!(
        result,
        Err(MemoryEnvelopeError::ScopeNotAncestor { .. })
    ));
    Ok(())
}

#[test]
fn changed_swap_allowance_and_ambiguous_membership_fail_closed() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc = tempfile::tempdir()?;
    let membership = proc.path().join("cgroup");
    let scope = write_group(
        mount.path(),
        "run",
        b"100\n",
        b"100\n",
        b"1073741824\n",
        b"0\n",
    )?;
    write_membership(&membership, "/run")?;
    let reader =
        CgroupMemoryReader::with_paths(gib_bytes(1)?, membership.clone(), mount.path().into())?;
    std::fs::write(scope.join("memory.swap.max"), b"1\n")?;
    check!(matches!(
        reader.sample(),
        Err(MemoryEnvelopeError::NonZeroSwap { .. })
    ));
    std::fs::write(&membership, b"0::/run\n0::/other\n")?;
    check!(matches!(
        reader.sample(),
        Err(MemoryEnvelopeError::MembershipMalformed { .. })
    ));
    Ok(())
}

#[test]
fn counter_ceiling_is_inclusive_and_signed_or_overflowed_values_are_refused() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc = tempfile::tempdir()?;
    let membership = proc.path().join("cgroup");
    let scope = write_group(
        mount.path(),
        "run",
        b"100\n",
        b"100\n",
        b"1073741824\n",
        b"0\n",
    )?;
    write_membership(&membership, "/run")?;
    let reader = CgroupMemoryReader::with_paths(gib_bytes(1)?, membership, mount.path().into())?;
    std::fs::write(scope.join("memory.current"), vec![b'0'; 4096])?;
    check!(eq; reader.sample()?.current_bytes, 0);
    std::fs::write(scope.join("memory.current"), vec![b'0'; 4097])?;
    check!(matches!(
        reader.sample(),
        Err(MemoryEnvelopeError::CounterOversized { .. })
    ));
    for raw in [
        b"+1".as_slice(),
        b"-1",
        b"",
        b"18446744073709551616",
        b"1 2",
    ] {
        std::fs::write(scope.join("memory.current"), raw)?;
        check!(matches!(
            reader.sample(),
            Err(MemoryEnvelopeError::CounterMalformed { .. })
        ));
    }
    Ok(())
}
