use super::super::TestResult;
use super::reaper;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;

const READY_ATTEMPTS: usize = 240;
const READY_DELAY: Duration = Duration::from_millis(250);
const SHUTDOWN_ATTEMPTS: usize = 300;
const SHUTDOWN_DELAY: Duration = Duration::from_millis(100);

pub(super) struct NativeNode {
    binary: PathBuf,
    config: PathBuf,
    root: Option<tempfile::TempDir>,
    root_path: PathBuf,
    pub(super) admin: String,
    pub(super) ingress: String,
    child: Option<Child>,
}

impl NativeNode {
    pub(super) fn new() -> TestResult<Self> {
        let binary = pinned_server()?;
        let (node_port, ingress_port, admin_port) = free_ports()?;
        let root = tempfile::Builder::new()
            .prefix("native-restate-")
            .tempdir()?;
        let root_path = root.path().to_path_buf();
        let data = root_path.join("restate-data");
        std::fs::create_dir_all(&data)?;
        let config = root_path.join("restate.toml");
        std::fs::write(
            &config,
            format!(
                "roles = [\"http-ingress\", \"admin\", \"worker\", \"log-server\", \"metadata-server\"]\n\
                 node-name = \"selected-workbook-e2e\"\n\
                 cluster-name = \"selected-workbook-e2e\"\n\
                 auto-provision = true\n\
                 default-num-partitions = 1\n\
                 default-replication = 1\n\
                 base-dir = \"{}\"\n\
                 listen-mode = \"tcp\"\n\
                 bind-ip = \"127.0.0.1\"\n\
                 bind-port = {node_port}\n\
                 advertised-address = \"http://127.0.0.1:{node_port}/\"\n\
                 shutdown-timeout = \"30s\"\n\
                 disable-telemetry = true\n\
                 experimental-enable-protocol-v7 = true\n\
                 experimental-enable-vqueues = true\n\
                 experimental-enable-scoped-virtual-objects = true\n\
                 \n\
                 [bifrost]\n\
                 default-provider = \"replicated\"\n\
                 \n\
                 [worker]\n\
                 durability-mode = \"replica-set-only\"\n\
                 \n\
                 [admin]\n\
                 bind-port = {admin_port}\n\
                 advertised-address = \"http://127.0.0.1:{admin_port}/\"\n\
                 \n\
                 [ingress]\n\
                 bind-port = {ingress_port}\n",
                data.display(),
            ),
        )?;
        Ok(Self {
            binary,
            config,
            root: Some(root),
            root_path,
            admin: format!("http://127.0.0.1:{admin_port}"),
            ingress: format!("http://127.0.0.1:{ingress_port}"),
            child: None,
        })
    }

    pub(super) fn start(&mut self) -> TestResult {
        check!(
            self.child.is_none(),
            "native Restate node is already running"
        );
        check!(
            self.root.is_some(),
            "native Restate node root ownership was handed off after cleanup failure"
        );
        match self.spawn() {
            Ok(child) => self.child = Some(child),
            Err(error) => {
                let failure =
                    self.preserve_failure(&format!("native Restate spawn failed: {error}"));
                return Err(failure.into());
            }
        }
        Ok(())
    }

    fn spawn(&self) -> std::io::Result<Child> {
        let log_path = self.root_path.join("restate-server.log");
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        Command::new(&self.binary)
            .arg("--no-logo")
            .arg("--config-file")
            .arg(&self.config)
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log))
            .spawn()
    }

    pub(super) async fn restart(&mut self, client: &reqwest::Client) -> TestResult {
        self.stop().await?;
        self.start()?;
        self.wait_ready(client).await
    }

    pub(super) async fn wait_ready(&mut self, client: &reqwest::Client) -> TestResult {
        let mut last = String::from("no readiness request made");
        for _ in 0..READY_ATTEMPTS {
            self.require_running()?;
            match client
                .post(format!("{}/query", self.admin))
                .header("accept", "application/json")
                .json(&serde_json::json!({
                    "query": "SELECT id FROM sys_invocation LIMIT 1"
                }))
                .timeout(Duration::from_secs(2))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response) => last = format!("admin query answered {}", response.status()),
                Err(error) => last = error.to_string(),
            }
            tokio::time::sleep(READY_DELAY).await;
        }
        Err(self
            .stop_failure(&format!("node did not become ready: {last}"))
            .into())
    }

    pub(super) async fn stop(&mut self) -> TestResult {
        let Some(child) = self.child.as_mut() else {
            return Ok(());
        };
        match child.try_wait() {
            Ok(Some(status)) => return self.stopped(status),
            Ok(None) => {}
            Err(error) => {
                return Err(self
                    .stop_failure(&format!("graceful stop inspection failed: {error}"))
                    .into());
            }
        }
        if let Err(error) = reaper::request_sigterm(child) {
            return Err(self
                .stop_failure(&format!("graceful stop request failed: {error}"))
                .into());
        }
        for _ in 0..SHUTDOWN_ATTEMPTS {
            let status = self
                .child
                .as_mut()
                .ok_or("native Restate child ownership lost during stop")?
                .try_wait();
            match status {
                Ok(Some(status)) => return self.stopped(status),
                Ok(None) => tokio::time::sleep(SHUTDOWN_DELAY).await,
                Err(error) => {
                    return Err(self
                        .stop_failure(&format!("graceful stop inspection failed: {error}"))
                        .into());
                }
            }
        }
        Err(self
            .stop_failure("native Restate server exceeded the 30s shutdown timeout")
            .into())
    }

    fn stopped(&mut self, status: ExitStatus) -> TestResult {
        if !status.success() {
            return Err(self
                .stop_failure(&format!(
                    "native Restate server exited unsuccessfully: {status}"
                ))
                .into());
        }
        drop(self.child.take());
        Ok(())
    }

    fn stop_failure(&mut self, reason: &str) -> String {
        let handoff = match self.child.take() {
            Some(child) => self.handoff(child, reason, false),
            None => reason.to_owned(),
        };
        self.failure(&handoff)
    }

    fn require_running(&mut self) -> TestResult {
        let status = match self
            .child
            .as_mut()
            .ok_or("native Restate child is not running")?
            .try_wait()
        {
            Ok(status) => status,
            Err(error) => {
                return Err(self
                    .stop_failure(&format!("readiness inspection failed: {error}"))
                    .into());
            }
        };
        if let Some(status) = status {
            return Err(self
                .stop_failure(&format!("node exited before readiness: {status}"))
                .into());
        }
        Ok(())
    }

    fn preserve_failure(&mut self, reason: &str) -> String {
        if let Some(mut root) = self.root.take() {
            root.disable_cleanup(true);
        }
        self.failure(reason)
    }

    fn failure(&self, reason: &str) -> String {
        let log_path = self.root_path.join("restate-server.log");
        let log = match std::fs::read_to_string(&log_path) {
            Ok(log) => log,
            Err(error) => format!("server log unavailable: {error}"),
        };
        format!(
            "{reason}; pid={:?} path={} log={}:\n{log}",
            self.child.as_ref().map(Child::id),
            self.root_path.display(),
            log_path.display()
        )
    }
}

impl Drop for NativeNode {
    fn drop(&mut self) {
        match self.child.take() {
            Some(child) => {
                let handoff = self.handoff(
                    child,
                    "native Restate node dropped before explicit stop",
                    true,
                );
                eprintln!("{handoff}");
            }
            None => {
                if let Some(root) = self.root.take() {
                    reaper::cleanup_stopped(root);
                }
            }
        }
    }
}

impl NativeNode {
    fn handoff(&mut self, child: Child, reason: &str, cleanup_on_exit: bool) -> String {
        match self.root.take() {
            Some(root) => reaper::handoff(child, root, reason, cleanup_on_exit),
            None => format!(
                "{reason}; cleanup handoff unavailable pid={} path={} root ownership already transferred",
                child.id(),
                self.root_path.display()
            ),
        }
    }
}

fn pinned_server() -> TestResult<PathBuf> {
    const SERVER_ENV: &str = "RESTATE_SERVER_BIN";
    let configured = std::env::var_os(SERVER_ENV).map(PathBuf::from);
    let binary = match configured {
        Some(path) => path,
        None => {
            let home_candidate = std::env::var_os("HOME").map(|home| {
                PathBuf::from(home)
                    .join(".local/share/athletic-rust-pipeline/restate/1.7.10/restate-server")
            });
            let path_candidate = std::env::var_os("PATH").and_then(|path| {
                std::env::split_paths(&path)
                    .map(|directory| directory.join("restate-server"))
                    .find(|candidate| candidate.is_file())
            });
            home_candidate
                .filter(|candidate| candidate.is_file())
                .or(path_candidate)
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!(
                            "native Restate 1.7.10 binary not found; set {SERVER_ENV} to its path"
                        ),
                    )
                })?
        }
    };
    let binary = std::fs::canonicalize(binary)?;
    let metadata = std::fs::metadata(&binary)?;
    check!(
        metadata.is_file() && metadata.len() <= 512 * 1024 * 1024,
        "pinned Restate binary is not a regular file under 512 MiB"
    );
    let version = Command::new(&binary).arg("--version").output()?;
    check!(
        version.status.success(),
        "pinned Restate version query failed"
    );
    let version = String::from_utf8(version.stdout)?;
    check!(eq; version.trim(), "restate-server 1.7.10");
    Ok(binary)
}

fn free_ports() -> TestResult<(u16, u16, u16)> {
    let node = TcpListener::bind(("127.0.0.1", 0))?;
    let ingress = TcpListener::bind(("127.0.0.1", 0))?;
    let admin = TcpListener::bind(("127.0.0.1", 0))?;
    Ok((
        node.local_addr()?.port(),
        ingress.local_addr()?.port(),
        admin.local_addr()?.port(),
    ))
}
