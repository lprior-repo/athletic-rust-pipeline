use super::{configuration, equal, logical_image, profile, TestResult};
use census_store::Store;
use std::path::Path;
use std::process::Command;

const RETAINED_ARTIFACT_ROOTS: [&str; 6] =
    ["http", "out", "entities", "journal", "capture", "captures"];
const RETAINED_ARTIFACT_NAME: &str = ".retained.jsonl.4294967295.0.part";

fn retained_artifact_image(root: &Path) -> TestResult<Vec<(String, Vec<u8>)>> {
    use std::io::Read;
    let mut image = Vec::new();
    for name in RETAINED_ARTIFACT_ROOTS {
        let directory = root.join(name);
        equal(std::fs::symlink_metadata(&directory)?.is_dir(), true)?;
        let entries = std::fs::read_dir(&directory)?
            .take(2)
            .collect::<Result<Vec<_>, _>>()?;
        equal(entries.len(), 1)?;
        let entry = entries.first().ok_or("retained artifact entry")?;
        equal(
            entry.file_name(),
            std::ffi::OsString::from(RETAINED_ARTIFACT_NAME),
        )?;
        equal(entry.file_type()?.is_file(), true)?;
        let mut bytes = Vec::new();
        std::fs::File::open(entry.path())?
            .take(257)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 256 {
            return Err("retained artifact exceeds fixture byte bound".into());
        }
        image.push((name.to_string(), bytes));
    }
    Ok(image)
}

#[test]
fn actual_fetch_cli_refuses_enrollment_before_network_and_preserves_retained_state() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let profile = profile()?;
    store.register_profile(&profile)?;
    let identity = store.store_identity()?;
    drop(store);
    let config = configuration(&profile)?;
    let mut control = Store::installation_control_existing(root.path())?;
    census_service::installation::enroll(&mut control, &config)?;
    let enrollment = control.show()?;
    drop(control);
    for name in RETAINED_ARTIFACT_ROOTS {
        let directory = root.path().join(name);
        std::fs::create_dir_all(&directory)?;
        std::fs::write(
            directory.join(RETAINED_ARTIFACT_NAME),
            format!("retained {name} evidence\n"),
        )?;
    }
    let logical_before = logical_image(root.path())?;
    let artifacts_before = retained_artifact_image(root.path())?;
    let sentinel = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    sentinel.set_nonblocking(true)?;
    let url = format!("http://{}/never-acquire", sentinel.local_addr()?);
    let output = Command::new(env!("CARGO_BIN_EXE_census-service"))
        .current_dir(root.path())
        .env("RUST_LOG", "off")
        .env("RUST_BACKTRACE", "0")
        .env("RUST_LIB_BACKTRACE", "0")
        .arg("--store")
        .arg(root.path())
        .args(["--authorized-host", "127.0.0.1", "fetch", &url, "--refresh"])
        .output()?;
    let refusal = census_store::StoreError::Installation(
        census_store::InstallationError::EnrollmentOnlyUnsupported {
            store_identity: identity,
            config_id: config.id()?,
        },
    );
    equal(output.status.code(), Some(1))?;
    equal(output.stdout, Vec::<u8>::new())?;
    equal(
        String::from_utf8(output.stderr)?,
        format!("Error: {refusal}\n"),
    )?;
    match sentinel.accept() {
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
        Err(error) => return Err(error.into()),
        Ok(_) => return Err("acquisition contacted the loopback sentinel before refusal".into()),
    }
    equal(logical_image(root.path())?, logical_before)?;
    equal(retained_artifact_image(root.path())?, artifacts_before)?;
    equal(root.path().join("var").exists(), false)?;
    let control = Store::installation_control_existing(root.path())?;
    equal(control.show()?, enrollment)
}
