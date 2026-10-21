use super::{codec, TestResult};
use crate::restate_services::wire::TeamsSourceRequest;
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub(super) const KEY: &str = "jurisdiction:IL:2026-27:1/teams/ihsa";
pub(super) const INVOCATION: &str = "inv_13df6geO7MHf3iuTE4mZEh3prJPYziW3qV";
const CAPTURE: &[u8] = include_bytes!("ihsa_20261021.journal.json");
const DIGEST: &str = "f40469de654474778335dd135aacd1e9f855c7624ad8984d137e096d66d4d1b4";

pub(super) struct Prefix {
    pub(super) request: TeamsSourceRequest,
    pub(super) frames: Vec<codec::Frame>,
}

pub(super) struct Journal {
    pub(super) key: &'static str,
    pub(super) invocation: &'static str,
    pub(super) frames: Vec<codec::Frame>,
}

pub(super) struct CaptureSpec {
    pub(super) bytes: &'static [u8],
    pub(super) digest: &'static str,
    pub(super) observed_at: &'static str,
    pub(super) key: &'static str,
    pub(super) invocation: &'static str,
    pub(super) entries: usize,
}

impl Journal {
    pub(super) fn input<T: serde::de::DeserializeOwned>(&self) -> TestResult<T> {
        let input = self.frames.first().ok_or("retained Input is absent")?;
        Ok(serde_json::from_slice(
            codec::field(input.bytes(14)?, 1)?.ok_or("retained Input has no payload")?,
        )?)
    }
}

#[derive(Deserialize)]
struct Capture {
    observed_at: String,
    rows: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    id: String,
    index: usize,
    entry_type: String,
    name: Option<String>,
    raw: String,
    raw_length: usize,
    version: u8,
}

pub(super) fn read() -> TestResult<Prefix> {
    let retained = read_journal(CaptureSpec {
        bytes: CAPTURE,
        digest: DIGEST,
        observed_at: "2026-10-21T14:54:36.294Z",
        key: KEY,
        invocation: INVOCATION,
        entries: 14,
    })?;
    let request: TeamsSourceRequest = retained.input()?;
    check!(eq; request.source.as_str(), "ihsa");
    check!(eq; request.observed_on.as_str(), "2026-10-21");
    check!(eq; request.jurisdiction.source_parallelism, 4);
    let sleep = retained.frames.last().ok_or("retained Sleep is absent")?;
    check!(eq; sleep.kind, 0x040C);
    check!(eq; sleep.number(11)?, 7);
    Ok(Prefix {
        request,
        frames: retained.frames,
    })
}

pub(super) fn read_journal(spec: CaptureSpec) -> TestResult<Journal> {
    check!(eq; format!("{:x}", Sha256::digest(spec.bytes)), spec.digest);
    let capture: Capture = serde_json::from_slice(spec.bytes)?;
    check!(eq; capture.observed_at, spec.observed_at);
    check!(eq; capture.rows.len(), spec.entries);
    let frames = capture
        .rows
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            check!(eq; entry.id, spec.invocation);
            check!(eq; entry.index, index);
            check!(eq; entry.version, 2);
            let kind = match entry.entry_type.as_str() {
                "Command: Input" => 0x0400,
                "Command: GetEagerState" => 0x0407,
                "Command: SetState" => 0x0403,
                "Command: Call" => 0x040D,
                "Command: Run" => 0x0411,
                "Command: Sleep" => 0x040C,
                "Notification: CallInvocationId" => 0x800E,
                "Notification: Call" => 0x800D,
                "Notification: Run" => 0x8011,
                "Notification: Sleep" => 0x800C,
                other => return Err(format!("unsupported retained journal entry {other}").into()),
            };
            if kind < 0x8000 {
                check!(eq; entry.name.as_deref(), Some(""));
            } else {
                check!(entry.name.is_none());
            }
            let raw = decode_hex(&entry.raw)?;
            check!(eq; raw.len(), entry.raw_length);
            Ok(codec::Frame::new(kind, raw))
        })
        .collect::<TestResult<Vec<_>>>()?;
    check!(eq; frames.first().ok_or("retained journal is empty")?.kind, 0x0400);
    Ok(Journal {
        key: spec.key,
        invocation: spec.invocation,
        frames,
    })
}

fn decode_hex(raw: &str) -> TestResult<Vec<u8>> {
    check!(eq; raw.len() % 2, 0);
    raw.as_bytes()
        .chunks_exact(2)
        .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
        .collect()
}
