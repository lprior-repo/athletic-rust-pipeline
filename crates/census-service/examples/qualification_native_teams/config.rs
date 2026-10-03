use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{atomic::AtomicBool, Arc};

use super::artifacts::{now, write_json, write_new};
use super::process::{command, OwnedProcess};

pub struct Config {
    pub root: PathBuf,
    pub serve: PathBuf,
    pub native: PathBuf,
    pub endpoint_port: u16,
    pub ingress: String,
    pub admin: String,
    pub ports: Vec<u16>,
    reservations: Vec<TcpListener>,
    output_failure: Arc<AtomicBool>,
}

impl Config {
    pub fn prepare() -> Result<Self> {
        let mut args = std::env::args_os().skip(1);
        let root = PathBuf::from(args.next().context("pass a fresh owned root")?);
        let serve = std::fs::canonicalize(args.next().context("pass actual census-serve path")?)?;
        let native =
            std::fs::canonicalize(args.next().context("pass actual native Restate path")?)?;
        ensure!(
            args.next().is_none(),
            "usage: qualification_native_teams FRESH_ROOT CENSUS_SERVE NATIVE_RESTATE"
        );
        ensure!(
            serve.is_file() && native.is_file(),
            "binary paths must be files"
        );
        std::fs::create_dir(&root).context("fresh exclusive root required; reuse refused")?;
        let root = std::fs::canonicalize(root)?;
        write_json(
            &root.join("invocation.json"),
            &json!({"argv":std::env::args_os().map(|s| s.to_string_lossy().into_owned()).collect::<Vec<_>>(), "pid":std::process::id(), "started":now()?, "cwd":std::env::current_dir()?, "model":"openai-codex/gpt-6.1-sol"}),
        )?;
        let reservations = (0..4)
            .map(|_| TcpListener::bind("127.0.0.1:0"))
            .collect::<std::io::Result<Vec<_>>>()?;
        let ports = reservations
            .iter()
            .map(|s| s.local_addr().map(|a| a.port()))
            .collect::<std::io::Result<Vec<_>>>()?;
        let endpoint_port = *ports.get(3).context("endpoint port reservation absent")?;
        let ingress_port = ports.get(1).context("ingress port reservation absent")?;
        let admin_port = ports.get(2).context("admin port reservation absent")?;
        write_config(&root, &ports)?;
        Ok(Self {
            root,
            serve,
            native,
            endpoint_port,
            ingress: format!("http://127.0.0.1:{ingress_port}/"),
            admin: format!("http://127.0.0.1:{admin_port}/"),
            ports,
            reservations,
            output_failure: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn capability(&self) -> Result<Value> {
        let isolation = super::isolation::prepare(&self.root)?;
        let version = command(
            &self.root,
            "native-version",
            &self.native,
            &["--version".to_string()],
        )?;
        ensure!(version.contains("restate") && version.split_whitespace().any(|part| part.starts_with("1.7.")), "qualification requires measured native Restate 1.7.x for protocol v7; observed {version:?}");
        let evidence = json!({"isolation":isolation, "native_version_output":version, "accepted_version_family":"1.7.x", "sdk_contract":"0.12.0", "scope":"fresh native teams qualification, never national completeness"});
        write_json(&self.root.join("prerequisites.json"), &evidence)?;
        Ok(evidence)
    }

    pub fn start_node(&mut self) -> Result<OwnedProcess> {
        self.reservations.clear();
        OwnedProcess::spawn(
            &self.root,
            "restate-server",
            &self.native,
            &[
                "--no-logo".to_string(),
                "--config-file".to_string(),
                self.root.join("restate.toml").display().to_string(),
            ],
            &[("RUST_LOG", "warn")],
            Arc::clone(&self.output_failure),
        )
    }

    pub fn start_endpoint(&self) -> Result<OwnedProcess> {
        self.endpoint_named("census-serve")
    }

    pub fn restart_endpoint(&self) -> Result<OwnedProcess> {
        self.endpoint_named("census-serve-restarted")
    }

    fn endpoint_named(&self, name: &str) -> Result<OwnedProcess> {
        let args = vec![
            "--listen".into(),
            format!("127.0.0.1:{}", self.endpoint_port),
            "--data-dir".into(),
            self.root.join("store").display().to_string(),
            "--max-concurrent".into(),
            "2".into(),
            "--drain-timeout".into(),
            "30".into(),
        ];
        OwnedProcess::spawn(
            &self.root,
            name,
            &self.serve,
            &args,
            &[(
                "RUST_LOG",
                "census_service=info,restate_sdk=trace,restate_sdk_shared_core=trace",
            )],
            Arc::clone(&self.output_failure),
        )
    }
}

fn write_config(root: &Path, ports: &[u16]) -> Result<()> {
    let node = ports.first().context("node port absent")?;
    let ingress = ports.get(1).context("ingress port absent")?;
    let admin = ports.get(2).context("admin port absent")?;
    let name = format!("qualification-native-teams-{}", std::process::id());
    let base = serde_json::to_string(&root.join("restate").display().to_string())?;
    let config = format!("roles = [\"http-ingress\", \"admin\", \"worker\", \"log-server\", \"metadata-server\"]\nnode-name = \"{name}\"\ncluster-name = \"{name}\"\nauto-provision = true\ndefault-num-partitions = 1\ndefault-replication = 1\nbase-dir = {base}\nlisten-mode = \"tcp\"\nbind-ip = \"127.0.0.1\"\nbind-port = {node}\nadvertised-address = \"http://127.0.0.1:{node}/\"\nshutdown-timeout = \"1m\"\ndisable-telemetry = true\nexperimental-enable-protocol-v7 = true\nexperimental-enable-vqueues = true\nexperimental-enable-scoped-virtual-objects = true\n\n[ingress]\nbind-address = \"127.0.0.1:{ingress}\"\n\n[admin]\nbind-address = \"127.0.0.1:{admin}\"\n");
    write_new(&root.join("restate.toml"), config.as_bytes())
}
