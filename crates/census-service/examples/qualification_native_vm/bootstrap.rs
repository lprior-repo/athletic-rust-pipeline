use super::artifacts;
use anyhow::{ensure, Context, Result};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::JoinHandle;
use std::time::Duration;

pub struct Seed {
    pub port: u16,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<()>>>,
}

impl Seed {
    pub fn start(root: &Path, key: &str) -> Result<Self> {
        ensure!(
            !key.contains('\n') && key.starts_with("ssh-ed25519 "),
            "invalid owned SSH public key"
        );
        let seed = root.join("seed");
        std::fs::create_dir(&seed)?;
        let config = format!(
            "#cloud-config\ndisable_root: false\nssh_pwauth: false\nusers:\n  - name: root\n    lock_passwd: true\n    ssh_authorized_keys:\n      - {key}\nbootcmd:\n  - [systemctl, mask, --now, systemd-timesyncd.service, systemd-time-wait-sync.service, pacman-init.service]\npackage_update: false\npackage_upgrade: false\n"
        );
        artifacts::write(&seed.join("user-data"), config.as_bytes())?;
        artifacts::write(
            &seed.join("meta-data"),
            b"instance-id: native-vm-fixture-replay\nlocal-hostname: census-qualification\n",
        )?;
        artifacts::write(&seed.join("vendor-data"), b"#cloud-config\n")?;
        artifacts::write(&seed.join("network-config"), b"version: 2\nethernets:\n  ethernet:\n    match:\n      name: 'en*'\n    dhcp4: true\n")?;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let worker = std::thread::spawn(move || {
            (0..18_000).try_for_each(|_| {
                if stopping.load(Ordering::Acquire) {
                    return Ok(());
                }
                match listener.accept() {
                    Ok((socket, _)) => respond(socket, &seed),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(100));
                        Ok(())
                    }
                    Err(error) => Err(error.into()),
                }
            })
        });
        Ok(Self {
            port,
            stop,
            worker: Some(worker),
        })
    }

    pub fn stop(&mut self) -> Result<()> {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| anyhow::anyhow!("bootstrap worker panicked"))??;
        }
        Ok(())
    }
}

impl Drop for Seed {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("bootstrap cleanup failed: {error:#}");
        }
    }
}

fn respond(mut socket: TcpStream, root: &Path) -> Result<()> {
    socket.set_read_timeout(Some(Duration::from_secs(2)))?;
    socket.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut buffer = [0_u8; 4096];
    let count = socket.read(&mut buffer)?;
    let request = std::str::from_utf8(
        buffer
            .get(..count)
            .context("bootstrap request exceeds buffer")?,
    )?;
    let name = request
        .split_whitespace()
        .nth(1)
        .context("bootstrap request path absent")?;
    let name = name.strip_prefix('/').context("bootstrap path invalid")?;
    ensure!(
        ["user-data", "meta-data", "vendor-data", "network-config"].contains(&name),
        "unowned bootstrap path refused"
    );
    let bytes = artifacts::read(&root.join(name))?;
    write!(
        socket,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bytes.len()
    )?;
    socket.write_all(&bytes)?;
    Ok(())
}
