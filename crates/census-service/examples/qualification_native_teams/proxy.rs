use anyhow::{ensure, Result};
use serde_json::{json, Value};
use std::io::Read;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use super::artifacts::{append, now, read_bounded};

mod hold;

struct Control {
    stopped: AtomicBool,
    boundary: AtomicUsize,
    released: AtomicBool,
    physical_evidence: Mutex<()>,
}
pub struct RefusalProxy {
    pub uri: String,
    control: Arc<Control>,
    thread: Option<JoinHandle<Result<usize>>>,
    ledger: PathBuf,
}

impl RefusalProxy {
    pub fn start(root: &Path) -> Result<Self> {
        let listener = TcpListener::bind((super::isolation::FAULT_ADDRESS, 443))?;
        listener.set_nonblocking(true)?;
        let uri = format!("tcp://{}/", listener.local_addr()?);
        let ledger = root.join("physical-requests.jsonl");
        let control = Arc::new(Control {
            stopped: AtomicBool::new(false),
            boundary: AtomicUsize::new(0),
            released: AtomicBool::new(true),
            physical_evidence: Mutex::new(()),
        });
        let stop = Arc::clone(&control);
        let path = ledger.clone();
        let thread = std::thread::Builder::new()
            .name("qualification-refusal-proxy".into())
            .spawn(move || serve(listener, stop, &path))?;
        Ok(Self {
            uri,
            control,
            thread: Some(thread),
            ledger,
        })
    }

    pub fn observations(&self) -> Result<Vec<Value>> {
        let _snapshot = self
            .control
            .physical_evidence
            .lock()
            .map_err(|_| anyhow::anyhow!("owned transport evidence lock poisoned"))?;
        if !self.ledger.exists() {
            return Ok(Vec::new());
        }
        read_bounded(&self.ledger)?
            .lines()
            .map(|line| serde_json::from_str(line).map_err(Into::into))
            .collect()
    }

    pub fn arm_third(&self, key: &str) -> Result<Value> {
        let boundary = self
            .observations()?
            .len()
            .checked_add(3)
            .ok_or_else(|| anyhow::anyhow!("hold sequence overflow"))?;
        let armed = json!({"event":"hold_armed", "key":key, "at":now()?,
            "boundary_sequence":boundary, "initial_eof_admissions":2,
            "attempt_count_authority":"shared progress and cold ledger, never TLS count"});
        append(
            &self.ledger.with_file_name("transport-events.jsonl"),
            &armed,
        )?;
        self.control.released.store(false, Ordering::Release);
        self.control.boundary.store(boundary, Ordering::Release);
        Ok(armed)
    }

    pub fn release(&self) -> Result<Value> {
        let event = json!({"event":"hold_release_requested", "at":now()?});
        append(
            &self.ledger.with_file_name("transport-events.jsonl"),
            &event,
        )?;
        self.control.released.store(true, Ordering::Release);
        self.control.boundary.store(0, Ordering::Release);
        Ok(event)
    }

    pub fn stop(&mut self) -> Result<Value> {
        self.control.stopped.store(true, Ordering::Release);
        let Some(thread) = self.thread.take() else {
            return Ok(json!({"already_joined":true}));
        };
        let requests = thread
            .join()
            .map_err(|_| anyhow::anyhow!("owned refusal proxy thread panicked"))??;
        Ok(
            json!({"joined":true, "physical_admissions":requests, "upstream_requests":0, "at":now()?}),
        )
    }
}

impl Drop for RefusalProxy {
    fn drop(&mut self) {
        if self.thread.is_none() {
            return;
        }
        if let Err(error) = self.stop() {
            eprintln!("refusal proxy cleanup error: {error:#}");
        }
    }
}

fn serve(listener: TcpListener, control: Arc<Control>, ledger: &Path) -> Result<usize> {
    let mut admitted = 0_usize;
    (0..60_000)
        .take_while(|_| !control.stopped.load(Ordering::Acquire))
        .try_for_each(|_| -> Result<()> {
            match listener.accept() {
                Ok((stream, peer)) => {
                    ensure!(
                        peer.ip().is_loopback()
                            || peer.ip().to_string() == super::isolation::FAULT_ADDRESS,
                        "peer is not on isolated loopback fault route"
                    );
                    ensure!(admitted < 128, "physical proxy admission ceiling exceeded");
                    admitted = admitted.saturating_add(1);
                    refuse(stream, ledger, admitted, &control)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(error) => return Err(error.into()),
            }
            Ok(())
        })?;
    Ok(admitted)
}

fn refuse(mut stream: TcpStream, ledger: &Path, sequence: usize, control: &Control) -> Result<()> {
    let accepted_at = now()?;
    let peer = stream.peer_addr()?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut buffer = [0_u8; 8192];
    let observed = stream.read(&mut buffer);
    let length = observed.as_ref().map_or(0, |length| *length);
    let fragment = buffer.get(..length).map_or(&[][..], |bytes| bytes);
    let boundary = control.boundary.load(Ordering::Acquire);
    let held = boundary != 0 && sequence >= boundary;
    let recording = control
        .physical_evidence
        .lock()
        .map_err(|_| anyhow::anyhow!("owned transport evidence lock poisoned"))?;
    append(
        ledger,
        &json!({"sequence":sequence, "peer":peer.to_string(), "accepted_at":accepted_at,
            "refused_at":if held {None} else {Some(now()?)}, "bytes_observed":length,
            "first_bytes_hex":fragment.iter().take(64).map(|byte| format!("{byte:02x}")).collect::<String>(),
            "tls_handshake_record_observed":fragment.first() == Some(&22),
            "read_error":observed.as_ref().err().map(ToString::to_string), "response_status":null,
            "held":held, "forwarded":false,
            "meaning":"owned TLS boundary; forced EOF or bounded hold; never a publisher HTTP response"}),
    )?;
    drop(recording);
    observed?;
    if held {
        hold::wait(control, ledger, sequence)?;
    }
    stream.shutdown(Shutdown::Both)?;
    Ok(())
}
