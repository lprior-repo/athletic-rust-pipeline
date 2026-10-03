use super::{artifacts, cancellation};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

pub fn reset(root: &Path) -> Result<Value> {
    let stream = UnixStream::connect(root.join("qmp.sock"))?;
    stream.set_read_timeout(Some(Duration::from_millis(100)))?;
    stream.set_write_timeout(Some(Duration::from_millis(100)))?;
    let mut reader = BufReader::new(stream);
    let greeting = line(&mut reader)?;
    ensure!(greeting.get("QMP").is_some(), "not an owned QMP greeting");
    let capabilities = json!({"execute":"qmp_capabilities","id":"capabilities"});
    send(&mut reader, &capabilities)?;
    let accepted = collect(&mut reader, "capabilities", false)?;
    let command = json!({"execute":"system_reset","id":"owned-hard-reset"});
    let before = artifacts::now();
    send(&mut reader, &command)?;
    let replies = collect(&mut reader, "owned-hard-reset", true)?;
    let evidence = json!({"greeting":greeting,"capabilities":accepted,"command":command,"host_sent_at":before,"host_received_at":artifacts::now(),"replies":replies,"fault":"hard virtual hardware reset via owned QMP; not host power-loss proof"});
    artifacts::publish(&root.join("qmp-reset.json"), &evidence)?;
    Ok(evidence)
}

fn send(reader: &mut BufReader<UnixStream>, value: &Value) -> Result<()> {
    cancellation::checkpoint()?;
    serde_json::to_writer(reader.get_mut(), value)?;
    reader.get_mut().write_all(b"\n")?;
    reader.get_mut().flush()?;
    Ok(())
}

fn line(reader: &mut BufReader<UnixStream>) -> Result<Value> {
    let mut bytes = Vec::with_capacity(1024);
    (0..100)
        .find_map(|_| {
            if let Err(error) = cancellation::checkpoint() {
                return Some(Err(error));
            }
            let remaining = match u64::try_from(bytes.len())
                .ok()
                .and_then(|length| 65_537_u64.checked_sub(length))
            {
                Some(remaining) => remaining,
                None => return Some(Err(anyhow::anyhow!("QMP line exceeds 64 KiB"))),
            };
            match std::io::Read::take(&mut *reader, remaining).read_until(b'\n', &mut bytes) {
                Ok(0) => return Some(Err(anyhow::anyhow!("QMP peer closed before complete line"))),
                Ok(_) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::TimedOut
                            | std::io::ErrorKind::Interrupted
                    ) => {}
                Err(error) => return Some(Err(error.into())),
            }
            if bytes.len() > 65_536 {
                return Some(Err(anyhow::anyhow!("QMP line exceeds 64 KiB")));
            }
            (bytes.last() == Some(&b'\n'))
                .then(|| serde_json::from_slice(&bytes).map_err(Into::into))
        })
        .context("QMP line incomplete within bounded cancellation-aware deadline")?
}

fn collect(reader: &mut BufReader<UnixStream>, id: &str, reset: bool) -> Result<Vec<Value>> {
    let mut replies = Vec::with_capacity(16);
    let mut completed = false;
    let mut reset_seen = !reset;
    let started = Instant::now();
    (0..256)
        .find_map(|_| {
            if let Err(error) = cancellation::checkpoint() {
                return Some(Err(error));
            }
            if started.elapsed() >= Duration::from_secs(10) {
                return Some(Err(anyhow::anyhow!("QMP reply deadline expired")));
            }
            let value = match line(reader) {
                Ok(value) => value,
                Err(error) => return Some(Err(error)),
            };
            if value.get("id").and_then(Value::as_str) == Some(id) {
                if value.get("error").is_some() {
                    return Some(Err(anyhow::anyhow!("QMP command failed: {value}")));
                }
                completed = value.get("return").is_some();
            }
            reset_seen |= value.get("event").and_then(Value::as_str) == Some("RESET");
            replies.push(value);
            (completed && reset_seen).then_some(Ok(()))
        })
        .context("QMP reply/actual RESET event not observed within message budget")??;
    Ok(replies)
}
