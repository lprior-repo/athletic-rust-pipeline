use sha2::{Digest, Sha256};
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub struct Fixture {
    _temporary: tempfile::TempDir,
    pub root: PathBuf,
    pub bin: PathBuf,
    pub linker: PathBuf,
    pub command: Command,
}

fn installed(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = Command::new("bash")
        .args(["-c", "command -v \"$1\"", "fixture", name])
        .env_remove("BASH_ENV")
        .env_remove("ENV")
        .output()?;
    if !output.status.success() {
        return Err(format!("required fixture tool is unavailable: {name}").into());
    }
    Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()))
}

pub fn executable(path: &Path, contents: &str) -> TestResult {
    std::fs::write(path, contents)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

impl Fixture {
    pub fn new(host: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temporary = tempfile::Builder::new().prefix("moon local ").tempdir()?;
        let base = temporary.path().canonicalize()?;
        let root = base.join("repository");
        let bin = base.join("bin");
        let sysroot = base.join("sysroot");
        std::fs::create_dir_all(root.join("tools"))?;
        std::fs::create_dir_all(&bin)?;
        for name in ["bash", "dirname", "jq", "mktemp", "mv", "rm", "shasum"] {
            symlink(installed(name)?, bin.join(name))?;
        }
        for name in [
            "rustup",
            "cargo",
            "rustc",
            "selected-rustc",
            "cargo-nextest",
            "moon",
            "cc",
            "c++",
            "ar",
            "selected-cc",
            "selected-c++",
            "selected-ar",
            "custom-ar",
            "xcrun",
            "ld",
            "workspace-wrapper",
        ] {
            executable(&bin.join(name), include_str!("command.sh"))?;
        }
        std::fs::write(
            root.join("tools/moon-local"),
            include_str!("../../../tools/moon-local"),
        )?;
        let linker = match host {
            "x86_64-unknown-linux-gnu" => {
                sysroot.join("lib/rustlib/x86_64-unknown-linux-gnu/bin/gcc-ld/ld.lld")
            }
            _ => bin.join("ld"),
        };
        std::fs::create_dir_all(linker.parent().ok_or("linker parent is missing")?)?;
        executable(&linker, "#!/usr/bin/env bash\nexit 0\n")?;
        let mut command = Command::new(installed("bash")?);
        command
            .arg("tools/moon-local")
            .current_dir(&root)
            .env_clear()
            .env("PATH", &bin)
            .env("HOME", &base)
            .env("CARGO_HOME", base.join("cargo-home"))
            .env("FIXTURE_BIN", &bin)
            .env("FIXTURE_SYSROOT", &sysroot)
            .env("FIXTURE_HOST", host)
            .env("FIXTURE_LINKER", &linker);
        Ok(Self {
            _temporary: temporary,
            root,
            bin,
            linker,
            command,
        })
    }

    pub fn fingerprint(&self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(&std::fs::read(
            self.root.join("tools/build-toolchain.json"),
        )?)?)
    }

    pub fn checksum_line(&self, path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        let digest = Sha256::digest(std::fs::read(path)?);
        Ok(format!("{digest:x}  {}", path.display()))
    }

    pub fn run(&mut self) -> Result<Output, Box<dyn std::error::Error>> {
        Ok(self.command.output()?)
    }
}
