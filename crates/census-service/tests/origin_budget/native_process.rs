use anyhow::{ensure, Result};
use serde::Serialize;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

#[derive(Debug, Serialize)]
pub(super) struct Owned {
    #[serde(skip)]
    child: Child,
    pub pid: u32,
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    pub root: PathBuf,
    pub ports: Vec<u16>,
    pub log: PathBuf,
}

impl Owned {
    pub(super) fn spawn(root: &Path, binary: &Path, argv: Vec<String>, ports: Vec<u16>) -> Result<Self> {
        let log = root.with_extension("log");
        let file = std::fs::OpenOptions::new().create_new(true).write(true).open(&log)?;
        let child = Command::new(binary).args(&argv).stdin(Stdio::null())
            .stdout(Stdio::from(file.try_clone()?)).stderr(Stdio::from(file)).spawn()?;
        Ok(Self { pid: child.id(), child, argv, exit_code: None, root: root.into(), ports, log })
    }

    pub(super) fn endpoint(directory: &Path, name: &str, port: u16) -> Result<Self> {
        let root = directory.join(format!("{name}-endpoint"));
        let log = directory.join(format!("{name}-endpoint.log"));
        let file = std::fs::OpenOptions::new().create_new(true).write(true).open(&log)?;
        let argv = vec!["--exact".to_string(), "native_budget_endpoint_child".to_string(), "--nocapture".to_string()];
        let child = Command::new(std::env::current_exe()?).args(&argv).current_dir(directory)
            .env("CENSUS_BUDGET_FIXTURE_STORE", &root)
            .env("CENSUS_BUDGET_FIXTURE_LISTEN", format!("127.0.0.1:{port}"))
            .env("CENSUS_BUDGET_FIXTURE_AGENT", format!("CensusOriginBudgetQualification/{name}"))
            .stdin(Stdio::null()).stdout(Stdio::from(file.try_clone()?)).stderr(Stdio::from(file)).spawn()?;
        Ok(Self { pid: child.id(), child, argv, exit_code: None, root, ports: vec![port], log })
    }

    pub(super) fn require_live(&mut self) -> Result<()> {
        ensure!(self.child.try_wait()?.is_none(), "owned process {} died; see {}", self.pid, self.log.display());
        Ok(())
    }

    pub(super) async fn stop(&mut self) -> Result<()> {
        if self.exit_code.is_some() { return Ok(()); }
        self.require_live()?;
        let status = Command::new("kill").args(["-TERM", &self.pid.to_string()]).status()?;
        ensure!(status.success(), "owned process TERM delivery failed: {status}");
        let mut ticks = tokio::time::interval(Duration::from_millis(100));
        for _ in 0..900 {
            if let Some(status) = self.child.try_wait()? {
                self.exit_code = status.code();
                ensure!(self.exit_code == Some(0), "owned process {} failed: {status}; {}", self.pid, self.log.display());
                return self.require_ports_released();
            }
            ticks.tick().await;
        }
        self.child.kill()?;
        let status = self.child.wait()?;
        Err(anyhow::anyhow!("owned process {} exceeded drain; emergency reaped {status}", self.pid))
    }

    fn require_ports_released(&self) -> Result<()> {
        for port in &self.ports {
            let reservation = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, *port))?;
            drop(reservation);
        }
        Ok(())
    }
}

pub(super) fn reserve(count: usize) -> Result<Vec<TcpListener>> {
    ensure!(count <= 16, "port reservation count exceeded");
    (0..count).map(|_| Ok(TcpListener::bind("127.0.0.1:0")?)).collect()
}

pub(super) fn pinned_binary() -> Result<PathBuf> {
    let path = match std::env::var_os("RESTATE_SERVER_BIN") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME missing"))?)
            .join(".local/share/athletic-rust-pipeline/restate/1.7.10/restate-server"),
    };
    let path = std::fs::canonicalize(path)?;
    let output = Command::new(&path).arg("--version").output()?;
    let version = String::from_utf8(output.stdout)?;
    ensure!(output.status.success() && version.split_whitespace().any(|word| word == "1.7.10"),
        "native Restate is not pinned 1.7.10: {version}");
    Ok(path)
}

pub(super) fn node(directory: &Path, name: &str, binary: &Path, ports: &[u16]) -> Result<Owned> {
    let node = *ports.first().ok_or_else(|| anyhow::anyhow!("node port missing"))?;
    let ingress = *ports.get(1).ok_or_else(|| anyhow::anyhow!("ingress port missing"))?;
    let admin = *ports.get(2).ok_or_else(|| anyhow::anyhow!("admin port missing"))?;
    let root = directory.join(format!("{name}-node"));
    let config = directory.join(format!("{name}-restate.toml"));
    let encoded_root = serde_json::to_string(&root.to_string_lossy())?;
    let text = format!("roles = [\"http-ingress\", \"admin\", \"worker\", \"log-server\", \"metadata-server\"]\nnode-name = \"budget-{name}\"\ncluster-name = \"budget-{name}\"\nauto-provision = true\ndefault-num-partitions = 1\ndefault-replication = 1\nbase-dir = {encoded_root}\nlisten-mode = \"tcp\"\nbind-ip = \"127.0.0.1\"\nbind-port = {node}\nadvertised-address = \"http://127.0.0.1:{node}/\"\nshutdown-timeout = \"1m\"\ndisable-telemetry = true\nexperimental-enable-protocol-v7 = true\nexperimental-enable-vqueues = true\nexperimental-enable-scoped-virtual-objects = true\n[bifrost]\ndefault-provider = \"replicated\"\n[worker]\ndurability-mode = \"replica-set-only\"\n[admin]\nbind-port = {admin}\nadvertised-address = \"http://127.0.0.1:{admin}/\"\n[ingress]\nbind-port = {ingress}\n");
    std::fs::write(&config, text)?;
    Owned::spawn(&root, binary, vec!["--no-logo".to_string(), "--config-file".to_string(), config.to_string_lossy().into_owned()], ports.to_vec())
}
