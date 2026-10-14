use super::CgroupMemoryReader;
use super::MemoryEnvelopeError;
use std::num::NonZeroU64;
use std::path::Path;
use std::path::PathBuf;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
#[cfg(target_os = "linux")]
fn gib_bytes(gib: u64) -> TestResult<NonZeroU64> {
    let bytes = gib
        .checked_mul(1024 * 1024 * 1024)
        .ok_or("budget overflow")?;
    let budget = NonZeroU64::new(bytes).ok_or("budget zero")?;
    Ok(budget)
}
#[cfg(target_os = "linux")]
fn write_group(
    mount: &Path,
    name: &str,
    current: &[u8],
    peak: &[u8],
    max: &[u8],
    swap: &[u8],
) -> TestResult<PathBuf> {
    let dir = mount.join(name);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("memory.current"), current)?;
    std::fs::write(dir.join("memory.peak"), peak)?;
    std::fs::write(dir.join("memory.max"), max)?;
    std::fs::write(dir.join("memory.swap.max"), swap)?;
    Ok(dir)
}
#[cfg(target_os = "linux")]
fn write_membership(proc_file: &Path, relative: &str) -> TestResult {
    let mut content = String::from("0::");
    content.push_str(relative);
    content.push('\n');
    std::fs::write(proc_file, content.as_bytes())?;
    Ok(())
}
#[cfg(target_os = "linux")]
#[test]
fn finite_envelope_reports_usage() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc_dir = tempfile::tempdir()?;
    let proc_file = proc_dir.path().join("cgroup");
    let scope = write_group(
        mount.path(),
        "group-a",
        b"104857600\n",
        b"209715200\n",
        b"1073741824\n",
        b"0\n",
    )?;
    write_membership(&proc_file, "/group-a")?;
    let budget = gib_bytes(2)?;
    let reader = CgroupMemoryReader::with_paths(budget, proc_file, mount.path().to_path_buf())?;
    let sample = reader.sample()?;
    check!(eq; sample.current_bytes, 104857600);
    check!(eq; sample.peak_bytes, 209715200);
    check!(eq; sample.hard_limit_bytes.get(), 1073741824);
    check!(eq; sample.scope, scope);
    Ok(())
}
#[cfg(target_os = "linux")]
#[test]
fn malformed_current_rejected() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc_dir = tempfile::tempdir()?;
    let proc_file = proc_dir.path().join("cgroup");
    write_group(
        mount.path(),
        "group-a",
        b"12x34\n",
        b"100\n",
        b"1073741824\n",
        b"0\n",
    )?;
    write_membership(&proc_file, "/group-a")?;
    let budget = gib_bytes(2)?;
    let reader = CgroupMemoryReader::with_paths(budget, proc_file, mount.path().to_path_buf())?;
    check!(matches!(
        reader.sample(),
        Err(MemoryEnvelopeError::CounterMalformed { .. })
    ));
    Ok(())
}
#[cfg(target_os = "linux")]
#[test]
fn malformed_membership_rejected() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc_dir = tempfile::tempdir()?;
    let proc_file = proc_dir.path().join("cgroup");
    write_group(
        mount.path(),
        "group-a",
        b"100\n",
        b"100\n",
        b"1073741824\n",
        b"0\n",
    )?;
    std::fs::write(&proc_file, b"1:name:/group-a\n")?;
    let budget = gib_bytes(2)?;
    check!(matches!(
        CgroupMemoryReader::with_paths(budget, proc_file, mount.path().to_path_buf()),
        Err(MemoryEnvelopeError::MembershipMalformed { .. })
    ));
    Ok(())
}
#[cfg(target_os = "linux")]
#[test]
fn missing_counter_rejected() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc_dir = tempfile::tempdir()?;
    let proc_file = proc_dir.path().join("cgroup");
    let scope = write_group(
        mount.path(),
        "group-a",
        b"100\n",
        b"100\n",
        b"1073741824\n",
        b"0\n",
    )?;
    write_membership(&proc_file, "/group-a")?;
    std::fs::remove_file(scope.join("memory.current"))?;
    let budget = gib_bytes(2)?;
    let reader = CgroupMemoryReader::with_paths(budget, proc_file, mount.path().to_path_buf())?;
    check!(matches!(
        reader.sample(),
        Err(MemoryEnvelopeError::CounterMissing { .. })
    ));
    Ok(())
}
#[cfg(target_os = "linux")]
#[test]
fn unbounded_max_rejected() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc_dir = tempfile::tempdir()?;
    let proc_file = proc_dir.path().join("cgroup");
    write_group(
        mount.path(),
        "group-a",
        b"100\n",
        b"100\n",
        b"max\n",
        b"0\n",
    )?;
    write_membership(&proc_file, "/group-a")?;
    let budget = gib_bytes(2)?;
    check!(matches!(
        CgroupMemoryReader::with_paths(budget, proc_file, mount.path().to_path_buf()),
        Err(MemoryEnvelopeError::UnboundedLimit { .. })
    ));
    Ok(())
}
#[cfg(target_os = "linux")]
#[test]
fn nonzero_swap_rejected() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc_dir = tempfile::tempdir()?;
    let proc_file = proc_dir.path().join("cgroup");
    write_group(
        mount.path(),
        "group-a",
        b"100\n",
        b"100\n",
        b"1073741824\n",
        b"4096\n",
    )?;
    write_membership(&proc_file, "/group-a")?;
    let budget = gib_bytes(2)?;
    check!(matches!(
        CgroupMemoryReader::with_paths(budget, proc_file.clone(), mount.path().to_path_buf()),
        Err(MemoryEnvelopeError::NonZeroSwap { .. })
    ));
    std::fs::write(
        mount.path().join("group-a").join("memory.swap.max"),
        b"max\n",
    )?;
    check!(matches!(
        CgroupMemoryReader::with_paths(budget, proc_file, mount.path().to_path_buf()),
        Err(MemoryEnvelopeError::NonZeroSwap { .. })
    ));
    Ok(())
}
#[cfg(target_os = "linux")]
#[test]
fn oversized_counter_rejected() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc_dir = tempfile::tempdir()?;
    let proc_file = proc_dir.path().join("cgroup");
    let big = vec![b'0'; 5000];
    write_group(
        mount.path(),
        "group-a",
        &big,
        b"100\n",
        b"1073741824\n",
        b"0\n",
    )?;
    write_membership(&proc_file, "/group-a")?;
    let budget = gib_bytes(2)?;
    let reader = CgroupMemoryReader::with_paths(budget, proc_file, mount.path().to_path_buf())?;
    check!(matches!(
        reader.sample(),
        Err(MemoryEnvelopeError::CounterOversized { .. })
    ));
    Ok(())
}
#[cfg(target_os = "linux")]
#[test]
fn membership_move_rejected() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc_dir = tempfile::tempdir()?;
    let proc_file = proc_dir.path().join("cgroup");
    write_group(
        mount.path(),
        "group-a",
        b"100\n",
        b"100\n",
        b"1073741824\n",
        b"0\n",
    )?;
    write_group(
        mount.path(),
        "group-b",
        b"100\n",
        b"100\n",
        b"1073741824\n",
        b"0\n",
    )?;
    write_membership(&proc_file, "/group-a")?;
    let budget = gib_bytes(2)?;
    let reader =
        CgroupMemoryReader::with_paths(budget, proc_file.clone(), mount.path().to_path_buf())?;
    write_membership(&proc_file, "/group-b")?;
    check!(matches!(
        reader.sample(),
        Err(MemoryEnvelopeError::MovedScope { .. })
    ));
    Ok(())
}
#[cfg(target_os = "linux")]
#[test]
fn path_traversal_rejected() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc_dir = tempfile::tempdir()?;
    let proc_file = proc_dir.path().join("cgroup");
    write_group(
        mount.path(),
        "group-a",
        b"100\n",
        b"100\n",
        b"1073741824\n",
        b"0\n",
    )?;
    write_membership(&proc_file, "/../escape")?;
    let budget = gib_bytes(2)?;
    check!(matches!(
        CgroupMemoryReader::with_paths(budget, proc_file, mount.path().to_path_buf()),
        Err(MemoryEnvelopeError::MembershipMalformed { .. })
    ));
    Ok(())
}
#[cfg(target_os = "linux")]
#[test]
fn peak_above_max_rejected() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc_dir = tempfile::tempdir()?;
    let proc_file = proc_dir.path().join("cgroup");
    write_group(
        mount.path(),
        "group-a",
        b"100\n",
        b"2147483648\n",
        b"1073741824\n",
        b"0\n",
    )?;
    write_membership(&proc_file, "/group-a")?;
    let budget = gib_bytes(2)?;
    let reader = CgroupMemoryReader::with_paths(budget, proc_file, mount.path().to_path_buf())?;
    check!(matches!(
        reader.sample(),
        Err(MemoryEnvelopeError::LimitExceeded { .. })
    ));
    Ok(())
}
#[cfg(target_os = "linux")]
#[test]
fn hard_limit_above_budget_rejected() -> TestResult {
    let mount = tempfile::tempdir()?;
    let proc_dir = tempfile::tempdir()?;
    let proc_file = proc_dir.path().join("cgroup");
    write_group(
        mount.path(),
        "group-a",
        b"100\n",
        b"100\n",
        b"2147483648\n",
        b"0\n",
    )?;
    write_membership(&proc_file, "/group-a")?;
    let budget = gib_bytes(1)?;
    check!(matches!(
        CgroupMemoryReader::with_paths(budget, proc_file, mount.path().to_path_buf()),
        Err(MemoryEnvelopeError::HardLimitAboveBudget { .. })
    ));
    Ok(())
}
#[cfg(not(target_os = "linux"))]
#[test]
fn unsupported_platform_rejected() -> TestResult {
    let budget = NonZeroU64::new(1073741824).ok_or("budget zero")?;
    check!(matches!(
        CgroupMemoryReader::declared(budget, Path::new("/")),
        Err(MemoryEnvelopeError::UnsupportedPlatform)
    ));
    Ok(())
}

#[cfg(target_os = "linux")]
mod envelope;
