use super::*;
use std::io::Write;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const HOLDER_ROOT: &str = "CENSUS_ORIGIN_LOCK_HOLDER_ROOT";
const HOLDER_READY: &str = "CENSUS_ORIGIN_LOCK_HOLDER_READY";
const RIVAL_TEST: &str =
    "net::origin_locks::tests::a_rival_process_is_refused_and_named_the_holder";

fn root(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("locks")
}

fn write_foreign_hold(root: &Path, origin: &str, record: &[u8]) -> TestResult<File> {
    std::fs::create_dir_all(root)?;
    let path = root.join(origin_lock_file_name(origin));
    let mut holder = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)?;
    holder.try_lock()?;
    holder.write_all(record)?;
    Ok(holder)
}

#[test]
fn a_foreign_hold_is_refused_and_the_record_is_named() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = root(&dir);
    let _holder = write_foreign_hold(
        &root,
        "https://cifsshome.org",
        br#"{"pid":424242,"origin":"https://cifsshome.org"}"#,
    )?;
    let locks = OriginLocks::rooted(root);
    let error = match locks.ensure("https://cifsshome.org") {
        Err(error) => error,
        Ok(()) => return Err("a foreign hold was adopted".into()),
    };
    match error {
        FetchError::OriginHeld { origin, holder } => {
            check!(eq; origin, "https://cifsshome.org");
            check!(holder.contains("424242"));
        }
        other => return Err(format!("unexpected error {other:?}").into()),
    }
    Ok(())
}

#[test]
fn a_rival_process_is_refused_and_named_the_holder() -> TestResult {
    if let Some(root) = std::env::var_os(HOLDER_ROOT) {
        return hold_until_killed(&root);
    }
    let dir = tempfile::tempdir()?;
    let root = root(&dir);
    let ready = dir.path().join("ready");
    let mut child = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", RIVAL_TEST, "--nocapture"])
        .env(HOLDER_ROOT, &root)
        .env(HOLDER_READY, &ready)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !ready.exists() {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            return Err("the rival process never took the hold".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let locks = OriginLocks::rooted(root.clone());
    let error = match locks.ensure("https://cifsshome.org") {
        Err(error) => error,
        Ok(()) => return Err("a rival process hold was adopted".into()),
    };
    let named = match error {
        FetchError::OriginHeld { origin, holder } => {
            check!(eq; origin, "https://cifsshome.org");
            holder
        }
        other => return Err(format!("unexpected error {other:?}").into()),
    };
    check!(named.contains(&format!("\"pid\":{}", child.id())));
    child.kill()?;
    child.wait()?;
    let released = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let probe = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join(origin_lock_file_name("https://cifsshome.org")))?;
        match probe.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) if std::time::Instant::now() < released => {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Err(error) => return Err(format!("the hold outlived its holder: {error:?}").into()),
        }
    }
    Ok(())
}

fn hold_until_killed(root: &std::ffi::OsStr) -> TestResult {
    let locks = OriginLocks::rooted(PathBuf::from(root));
    locks.ensure("https://cifsshome.org")?;
    if let Some(ready) = std::env::var_os(HOLDER_READY) {
        std::fs::write(ready, b"ready")?;
    }
    std::thread::sleep(std::time::Duration::from_secs(30));
    Ok(())
}

#[test]
fn one_instance_reuses_its_own_hold_and_drop_releases_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    {
        let locks = OriginLocks::rooted(root(&dir));
        locks.ensure("https://example.test")?;
        locks.ensure("https://example.test")?;
        let probe = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root(&dir).join(origin_lock_file_name("https://example.test")))?;
        check!(probe.try_lock().is_err());
    }
    let after = OriginLocks::rooted(root(&dir));
    after.ensure("https://example.test")?;
    Ok(())
}

#[test]
fn a_second_instance_in_one_process_adopts_its_own_hold() -> TestResult {
    let dir = tempfile::tempdir()?;
    let first = OriginLocks::rooted(root(&dir));
    let second = OriginLocks::rooted(root(&dir));
    first.ensure("https://example.test")?;
    second.ensure("https://example.test")?;
    Ok(())
}

#[test]
fn a_disabled_registry_locks_nothing() -> TestResult {
    let locks = OriginLocks::disabled();
    locks.ensure("example.test")?;
    check!(locks.ensure("").is_ok());
    Ok(())
}

#[test]
fn lock_files_are_sanitized_and_distinct_per_origin() -> TestResult {
    let dir = tempfile::tempdir()?;
    let locks = OriginLocks::rooted(root(&dir));
    locks.ensure("https://2001:db8::1")?;
    locks.ensure("https://example.test")?;
    let mut names: Vec<String> = std::fs::read_dir(root(&dir))?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<Vec<String>, std::io::Error>>()?;
    names.sort();
    check!(eq; names, vec![
        "https___2001_db8__1.lock".to_string(),
        "https___example.test.lock".to_string()
    ]);
    Ok(())
}

#[test]
fn an_unusable_root_reports_lock_io() -> TestResult {
    let dir = tempfile::tempdir()?;
    let blocker = dir.path().join("locks");
    std::fs::write(&blocker, b"not a directory")?;
    let locks = OriginLocks::rooted(blocker);
    match locks.ensure("example.test") {
        Err(FetchError::OriginLockIo { .. }) => Ok(()),
        other => Err(format!("unexpected result {other:?}").into()),
    }
}
