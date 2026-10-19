use super::super::TestResult;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const READY_ATTEMPTS: usize = 240;
const READY_DELAY: Duration = Duration::from_millis(250);
const DROP_SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

pub(super) struct NativeNode {
    binary: PathBuf,
    config: PathBuf,
    root: PathBuf,
    pub(super) admin: String,
    pub(super) ingress: String,
    child: Option<Child>,
}

impl NativeNode {
    pub(super) fn new(root: &Path) -> TestResult<Self> {
        let binary = pinned_server()?;
        let (node_port, ingress_port, admin_port) = free_ports()?;
        let data = root.join("restate-data");
        std::fs::create_dir_all(&data)?;
        let config = root.join("restate.toml");
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
            root: root.to_path_buf(),
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
        let log_path = self.root.join("restate-server.log");
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        self.child = Some(
            Command::new(&self.binary)
                .arg("--no-logo")
                .arg("--config-file")
                .arg(&self.config)
                .stdout(Stdio::from(log.try_clone()?))
                .stderr(Stdio::from(log))
                .spawn()?,
        );
        Ok(())
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
            .failure(&format!("node did not become ready: {last}"))
            .into())
    }

    pub(super) async fn stop(&mut self) -> TestResult {
        let Some(child) = self.child.as_mut() else {
            return Ok(());
        };
        request_sigterm(child)?;
        for _ in 0..300 {
            let status = self
                .child
                .as_mut()
                .ok_or("native Restate child disappeared during shutdown")?
                .try_wait()?;
            if let Some(status) = status {
                self.child = None;
                check!(
                    status.success(),
                    "native Restate server exited unsuccessfully: {status}"
                );
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(self
            .failure("native Restate server did not stop after SIGTERM")
            .into())
    }

    fn require_running(&mut self) -> TestResult {
        let status = self
            .child
            .as_mut()
            .ok_or("native Restate child is not running")?
            .try_wait()?;
        check!(
            status.is_none(),
            "{}",
            self.failure("node exited before readiness")
        );
        Ok(())
    }

    fn failure(&self, reason: &str) -> String {
        let log_path = self.root.join("restate-server.log");
        let log = match std::fs::read_to_string(&log_path) {
            Ok(log) => log,
            Err(error) => format!("server log unavailable: {error}"),
        };
        format!("{reason}; log={}:\n{log}", log_path.display())
    }
}

impl Drop for NativeNode {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        match child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                reap_owned_child(&mut child);
            }
            Err(error) => {
                eprintln!("could not inspect Restate child during cleanup: {error}");
                reap_owned_child(&mut child);
            }
        }
    }
}

fn request_sigterm(child: &Child) -> std::io::Result<()> {
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(child.id().to_string())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "kill -TERM exited with {status}"
        )))
    }
}

fn reap_owned_child(child: &mut Child) {
    if let Err(error) = request_sigterm(child) {
        eprintln!("could not request graceful Restate cleanup: {error}");
    }
    let deadline = Instant::now() + DROP_SHUTDOWN_GRACE;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Ok(None) => break,
            Err(error) => {
                eprintln!("could not inspect Restate child during cleanup: {error}");
                break;
            }
        }
    }
    if let Err(error) = child.kill() {
        eprintln!("could not terminate owned Restate child after grace period: {error}");
    }
    if let Err(error) = child.wait() {
        eprintln!("could not reap owned Restate child: {error}");
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
