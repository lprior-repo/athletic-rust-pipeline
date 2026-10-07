use super::{artifacts, cancellation, process};
use anyhow::{ensure, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

pub struct Ssh {
    pub root: PathBuf,
    pub port: u16,
}

impl Ssh {
    pub fn checkpoint(&self) -> Result<()> {
        cancellation::checkpoint()
    }

    pub fn pause(&self, duration: Duration) -> Result<()> {
        cancellation::pause(duration)
    }

    fn options(&self, command: &mut Command) {
        command
            .args([
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=3",
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
            ])
            .arg(format!(
                "UserKnownHostsFile={}",
                self.root.join("known_hosts").display()
            ))
            .arg("-i")
            .arg(self.root.join("ssh-key"));
    }

    pub fn exec(&self, label: &str, remote: &str) -> Result<String> {
        self.exec_bounded(label, remote, 1800)
    }

    fn exec_bounded(&self, label: &str, remote: &str, ticks: usize) -> Result<String> {
        let mut command = Command::new("/usr/bin/ssh");
        self.options(&mut command);
        command.args(["-p", &self.port.to_string(), "root@127.0.0.1", remote]);
        process::command(&self.root, label, &mut command, ticks)
    }

    pub fn action(&self, action: &str) -> Result<Value> {
        let remote = action_remote(action)?;
        if action == "ready" {
            self.wait_owner()?;
        }
        let ticks = match action {
            "jurisdiction-reboot-finish" => 35_000,
            _ => 1800,
        };
        let output = self.exec_bounded(action, &remote, ticks)?;
        serde_json::from_str(output.trim())
            .with_context(|| format!("guest {action} returned invalid evidence: {output}"))
    }

    pub fn clock_until(&self, deadline: Instant) -> Result<Value> {
        if Instant::now() >= deadline {
            return Err(process::DeadlineExpired.into());
        }
        let mut command = Command::new("/usr/bin/ssh");
        self.options(&mut command);
        command.args([
            "-p",
            &self.port.to_string(),
            "root@127.0.0.1",
            &action_remote("clock")?,
        ]);
        let output = process::command_until(&self.root, "clock", &mut command, deadline)?;
        let value = serde_json::from_str(output.trim())
            .with_context(|| format!("guest clock returned invalid evidence: {output}"))?;
        if Instant::now() >= deadline {
            return Err(process::DeadlineExpired.into());
        }
        Ok(value)
    }

    pub fn wait_boot(&self, previous: Option<&str>, owner: &mut process::Process) -> Result<Value> {
        (0..360).find_map(|attempt| {
            if let Err(error) = check_boot_owner(owner).and_then(|()| self.checkpoint()) {
                return Some(Err(error));
            }
            let response = self.exec_bounded("boot-probe", "/usr/bin/cat /proc/sys/kernel/random/boot_id", 100);
            if let Err(error) = check_boot_owner(owner) { return Some(Err(error)); }
            match response {
                Ok(text) if previous != Some(text.trim()) => Some(Ok(serde_json::json!({"boot_id":text.trim(),"host_observed_at":artifacts::now()}))),
                response => {
                    if let Err(error) = response { eprintln!("boot probe {attempt}: {error:#}"); }
                    let paused = check_boot_owner(owner)
                        .and_then(|()| self.pause(Duration::from_secs(2)))
                        .and_then(|()| check_boot_owner(owner));
                    match paused { Ok(()) => None, Err(error) => Some(Err(error)) }
                }
            }
        }).context("SSH/changed guest boot_id not observed within bounded boot budget")?
    }

    fn wait_owner(&self) -> Result<()> {
        (0..120)
            .find_map(|attempt| {
                if let Err(error) = self.checkpoint() {
                    return Some(Err(error));
                }
                let result = self.exec_bounded(
                    "service-owner-probe",
                    "/usr/bin/systemctl show --property MainPID --value qualification.service",
                    100,
                );
                match result {
                    Ok(pid) if pid.trim().parse::<u32>().is_ok_and(|pid| pid > 1) => Some(Ok(())),
                    Ok(_) => match self.pause(Duration::from_secs(1)) {
                        Ok(()) => None,
                        Err(error) => Some(Err(error)),
                    },
                    Err(error) => {
                        eprintln!("guest owner probe {attempt}: {error:#}");
                        match self.pause(Duration::from_secs(1)) {
                            Ok(()) => None,
                            Err(error) => Some(Err(error)),
                        }
                    }
                }
            })
            .context(
                "guest boot supervisor did not acquire a live service owner within fixed budget",
            )?
    }

    pub fn copy(&self, label: &str, sources: &[PathBuf], destination: &str) -> Result<()> {
        ensure!(
            !sources.is_empty() && sources.len() <= 128,
            "payload copy exceeds file budget"
        );
        let mut command = Command::new("/usr/bin/scp");
        self.options(&mut command);
        command
            .args(["-O", "-P", &self.port.to_string()])
            .args(sources)
            .arg(format!("root@127.0.0.1:{destination}"));
        process::command(&self.root, label, &mut command, 3000)?;
        Ok(())
    }
}

fn check_boot_owner(owner: &mut process::Process) -> Result<()> {
    let status = owner.health_status().with_context(|| {
        format!(
            "QEMU boot owner health failed; evidence={}; log={}",
            owner.evidence(),
            owner.log.display()
        )
    })?;
    ensure!(
        status.is_none(),
        "QEMU exited unexpectedly during guest boot: {status:?}; evidence={}; log={}",
        owner.evidence(),
        owner.log.display()
    );
    Ok(())
}

fn action_remote(action: &str) -> Result<String> {
    ensure!(
        action
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
        "invalid guest action"
    );
    Ok(format!(
        "/usr/bin/nsenter --net=/proc/$(/usr/bin/systemctl show --property MainPID --value qualification.service)/ns/net /srv/qualification/lib/ld-linux-x86-64.so.2 --library-path /srv/qualification/lib /srv/qualification/qualification guest-action --action {action}"
    ))
}

pub fn libraries(root: &Path, binaries: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut libraries = std::collections::BTreeSet::new();
    binaries
        .iter()
        .enumerate()
        .try_for_each(|(index, binary)| -> Result<()> {
            let output = process::command(
                root,
                &format!("ldd-{index}"),
                Command::new("/usr/bin/ldd").arg(binary),
                100,
            )?;
            ensure!(
                !output.contains("not found"),
                "missing dynamic library for {}: {output}",
                binary.display()
            );
            output
                .split_whitespace()
                .filter(|token| token.starts_with('/'))
                .try_for_each(|token| -> Result<()> {
                    let path = PathBuf::from(token);
                    ensure!(path.is_file(), "runtime library absent: {token}");
                    ensure!(
                        libraries.contains(&path) || libraries.len() < 128,
                        "dynamic dependency budget invalid"
                    );
                    libraries.insert(path);
                    Ok(())
                })
        })?;
    ensure!(
        !libraries.is_empty() && libraries.len() <= 128,
        "dynamic dependency budget invalid"
    );
    Ok(libraries.into_iter().collect())
}
