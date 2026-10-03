use super::{artifacts, process, GUEST};
use anyhow::{ensure, Context, Result};
use serde_json::json;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

mod enablement;
mod supervision;

pub fn initialize() -> Result<()> {
    let staging = Path::new("/root/qualification-payload");
    ensure!(
        !Path::new(GUEST).join("manifest.json").exists(),
        "guest initialization cannot replace recovery manifest"
    );
    let root = Path::new("/root");
    process::command(
        root,
        "disk-format",
        Command::new("/usr/bin/mkfs.btrfs").args(["-L", "QUALIFICATION", "/dev/vdb"]),
        600,
    )?;
    std::fs::create_dir_all(GUEST)?;
    process::command(
        root,
        "disk-mount",
        Command::new("/usr/bin/mount").args(["/dev/vdb", GUEST]),
        100,
    )?;
    std::fs::set_permissions(GUEST, std::fs::Permissions::from_mode(0o700))?;
    let uuid = process::command(
        root,
        "disk-uuid",
        Command::new("/usr/bin/blkid").args(["-s", "UUID", "-o", "value", "/dev/vdb"]),
        100,
    )?;
    ensure!(
        uuid.trim()
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() || byte == b'-'),
        "invalid owned disk UUID"
    );
    let fstab = std::fs::OpenOptions::new()
        .append(true)
        .open("/etc/fstab")?;
    use std::io::Write;
    writeln!(&fstab, "UUID={} {GUEST} btrfs defaults 0 0", uuid.trim())?;
    fstab.sync_all()?;
    copy_payload(staging, Path::new(GUEST))?;
    install_captures()?;
    artifacts::publish(
        &Path::new(GUEST).join("disk.json"),
        &json!({"uuid":uuid.trim(),"source":"/dev/vdb","mount":GUEST}),
    )?;
    write_units()?;
    process::command(
        root,
        "reload",
        Command::new("/usr/bin/systemctl").arg("daemon-reload"),
        100,
    )?;
    enablement::enable(root)
}

fn copy_payload(source: &Path, target: &Path) -> Result<()> {
    let entries = std::fs::read_dir(source)?
        .take(129)
        .collect::<std::io::Result<Vec<_>>>()?;
    ensure!(entries.len() <= 128, "payload file budget exceeded");
    entries.into_iter().try_for_each(|entry| -> Result<()> {
        let path = entry.path();
        let destination = target.join(entry.file_name());
        if path.is_dir() {
            ensure!(entry.file_name() == "lib", "unexpected payload directory");
            std::fs::create_dir(&destination)?;
            return copy_libraries(&path, &destination);
        }
        ensure!(path.is_file(), "non-file payload refused");
        std::fs::copy(path, &destination)?;
        std::fs::File::open(destination)?.sync_all()?;
        Ok(())
    })
}

fn copy_libraries(source: &Path, target: &Path) -> Result<()> {
    let files = std::fs::read_dir(source)?
        .take(129)
        .collect::<std::io::Result<Vec<_>>>()?;
    ensure!(files.len() <= 128, "runtime library budget exceeded");
    files.into_iter().try_for_each(|entry| -> Result<()> {
        ensure!(
            entry.path().is_file(),
            "runtime library must be regular file"
        );
        let destination = target.join(entry.file_name());
        std::fs::copy(entry.path(), &destination)?;
        std::fs::File::open(destination)?.sync_all()?;
        Ok(())
    })
}

fn install_captures() -> Result<()> {
    let cache = Path::new(GUEST).join("store/http");
    std::fs::create_dir_all(&cache)?;
    ["index", "roster"]
        .into_iter()
        .try_for_each(|name| -> Result<()> {
            let metadata = artifacts::json(&Path::new(GUEST).join(format!("{name}.meta.json")))?;
            let url = metadata
                .get("url")
                .and_then(serde_json::Value::as_str)
                .context("capture URL absent")?;
            let digest = artifacts::sha(format!("GET\u{1f}{url}\u{1f}").as_bytes());
            let key = digest.get(..32).context("capture key too short")?;
            artifacts::write(
                &cache.join(format!("{key}.body")),
                &artifacts::read(&Path::new(GUEST).join(format!("{name}.body")))?,
            )?;
            artifacts::publish(&cache.join(format!("{key}.meta.json")), &metadata)
        })
}

fn write_units() -> Result<()> {
    let unit = format!(
        "[Unit]\nDescription=Owned native fixture replay qualification\nRequiresMountsFor={GUEST}\nAfter=local-fs.target\n[Service]\nType=simple\nExecStart={GUEST}/lib/ld-linux-x86-64.so.2 --library-path {GUEST}/lib {GUEST}/qualification guest-supervise\nKillMode=mixed\nTimeoutStopSec=150\nSendSIGKILL=no\nRestart=on-failure\nRestartSec=2\nMemoryMax=3G\nTasksMax=128\nLimitNOFILE=4096\n[Install]\nWantedBy=multi-user.target\n"
    );
    artifacts::write(
        Path::new("/etc/systemd/system/qualification.service"),
        unit.as_bytes(),
    )
}

pub fn launch(binary: &str) -> Command {
    let mut command = Command::new(format!("{GUEST}/lib/ld-linux-x86-64.so.2"));
    command.args([
        "--library-path",
        &format!("{GUEST}/lib"),
        &format!("{GUEST}/{binary}"),
    ]);
    command
}

pub fn supervise() -> Result<()> {
    supervision::run()
}

fn verify_recovery_inputs(manifest: &serde_json::Value) -> Result<()> {
    let root = Path::new(GUEST);
    let binaries = manifest
        .get("binaries")
        .and_then(serde_json::Value::as_array)
        .context("durable binary identities absent")?;
    ensure!(binaries.len() == 3, "durable native binary set differs");
    binaries.iter().try_for_each(|binary| -> Result<()> {
        let name = binary
            .get("binary")
            .and_then(serde_json::Value::as_str)
            .context("binary name absent")?;
        ensure!(
            ["qualification", "census-serve", "restate-server"].contains(&name),
            "unowned binary identity"
        );
        let digest = super::host::file_sha(&root.join(name))?;
        ensure!(
            binary.get("sha256").and_then(serde_json::Value::as_str) == Some(digest.as_str()),
            "recovered native binary differs: {name}"
        );
        Ok(())
    })?;
    let digest = super::host::file_sha(&root.join("restate.toml"))?;
    ensure!(
        manifest
            .get("restate_config_sha256")
            .and_then(serde_json::Value::as_str)
            == Some(digest.as_str()),
        "recovered native configuration differs"
    );
    let disk = artifacts::json(&root.join("disk.json"))?;
    let uuid = process::command(
        root,
        "recovery-disk-uuid",
        Command::new("/usr/bin/blkid").args(["-s", "UUID", "-o", "value", "/dev/vdb"]),
        100,
    )?;
    ensure!(
        disk.get("uuid").and_then(serde_json::Value::as_str) == Some(uuid.trim()),
        "recovery data disk UUID differs"
    );
    let mounts = std::fs::read_to_string("/proc/mounts")?;
    ensure!(
        mounts.lines().any(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            fields.first().copied() == Some("/dev/vdb")
                && fields.get(1).copied() == Some(GUEST)
                && fields.get(2).copied() == Some("btrfs")
        }),
        "persistent guest data mount absent or replaced"
    );
    Ok(())
}
