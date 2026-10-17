use super::{format, inputs, BuildResult};
use std::env;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

pub(super) fn compiler() -> BuildResult<format::Input> {
    let rustc = env::var_os("RUSTC").ok_or("roster parser compiler path absent")?;
    let bytes = output(Command::new(rustc).arg("-vV"), 16 * 1024)?;
    if bytes.is_empty() {
        return Err("roster parser compiler identity absent".into());
    }
    inputs::digest("rustc-version-verbose", &bytes)
}

pub(super) fn vcs(root: &Path, manifest_sha256: String) -> BuildResult<format::Vcs> {
    inputs::watch(root.join(".git"))?;
    let (commit, state) = match known_vcs(root) {
        Ok(value) => value,
        Err(_) => (String::new(), format::RosterParserWorkingTreeV1::Unknown),
    };
    Ok(format::Vcs {
        manifest_sha256,
        commit,
        state,
    })
}

fn known_vcs(root: &Path) -> BuildResult<(String, format::RosterParserWorkingTreeV1)> {
    let bytes = output(
        Command::new("git")
            .current_dir(root)
            .args(["rev-parse", "--verify", "HEAD"]),
        128,
    )?;
    let commit = std::str::from_utf8(&bytes)?.trim();
    if ![40, 64].contains(&commit.len()) || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("roster parser Git commit invalid".into());
    }
    for name in ["HEAD", "index", "packed-refs", "refs"] {
        let path = output(
            Command::new("git")
                .current_dir(root)
                .args(["rev-parse", "--git-path", name]),
            4096,
        )?;
        inputs::watch(root.join(std::str::from_utf8(&path)?.trim()))?;
    }
    let mut command = Command::new("git");
    command.current_dir(root).args([
        "status",
        "--porcelain=v1",
        "--untracked-files=all",
        "--ignored=matching",
        "--",
    ]);
    command
        .args(format::SOURCE_ROOTS)
        .args(format::CONFIG_FILES)
        .arg("Cargo.lock");
    let status = output(&mut command, 1024 * 1024)?;
    let state = if status.is_empty() {
        format::RosterParserWorkingTreeV1::GitReportedClean
    } else {
        format::RosterParserWorkingTreeV1::GitReportedDirty
    };
    Ok((commit.into(), state))
}

fn output(command: &mut Command, limit: u64) -> BuildResult<Vec<u8>> {
    let mut bytes = Vec::new();
    let limit_usize = usize::try_from(limit)?;
    let read_limit = limit.checked_add(1).ok_or("process limit overflow")?;
    bytes.try_reserve_exact(usize::try_from(read_limit)?)?;
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let pipe = match child.stdout.take() {
        Some(pipe) => pipe,
        None => {
            child.wait()?;
            return Err("roster parser process stdout absent".into());
        }
    };
    let read = pipe.take(read_limit).read_to_end(&mut bytes);
    if read.is_err() || bytes.len() > limit_usize {
        child.wait()?;
        return Err("roster parser process output refused".into());
    }
    read?;
    if !child.wait()?.success() {
        return Err("roster parser provenance process failed".into());
    }
    Ok(bytes)
}
