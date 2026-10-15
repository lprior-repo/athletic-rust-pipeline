use super::{codec, TestResult};
use std::io::Write;
use std::path::{Path, PathBuf};

mod tests;

const MAX_TRANSCRIPT_BYTES: usize = 32 * 1024 * 1024;
const MAX_STATUS_BYTES: usize = 8192;
const PENDING: &[u8] = br#"{"transport":"pending"}"#;

pub(super) struct Transcript {
    root: PathBuf,
    phase: &'static str,
    sequence: u32,
    bytes: usize,
}

impl Transcript {
    pub(super) fn new(root: PathBuf) -> Self {
        Self {
            root,
            phase: "old",
            sequence: 0,
            bytes: 0,
        }
    }

    pub(super) fn candidate(&mut self) {
        self.phase = "current";
    }

    fn record(&mut self, request: &[u8], status: u16, response: &[u8]) -> TestResult {
        let stem = self.begin(request)?;
        self.finish(&stem, Some(status), response, None)
    }

    fn begin(&mut self, request: &[u8]) -> TestResult<String> {
        let reserved = request
            .len()
            .checked_add(codec::MAX_WIRE_BYTES)
            .and_then(|bytes| bytes.checked_add(MAX_STATUS_BYTES))
            .and_then(|bytes| bytes.checked_add(PENDING.len()))
            .ok_or("transcript reservation overflow")?;
        self.admit(reserved)?;
        check!(self.sequence < 256, "transcript exchange budget exhausted");
        let stem = format!("{}-{:04}", self.phase, self.sequence);
        write_new(&self.root.join(format!("{stem}.request.bin")), request)?;
        write_new(&self.root.join(format!("{stem}.pending.json")), PENDING)?;
        self.bytes += request.len() + PENDING.len();
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or("transcript sequence overflow")?;
        Ok(stem)
    }

    fn finish(
        &mut self,
        stem: &str,
        status: Option<u16>,
        response: &[u8],
        error: Option<String>,
    ) -> TestResult {
        let error = error.map(|message| message.chars().take(1024).collect::<String>());
        let status = serde_json::to_vec(&serde_json::json!({"status": status, "error": error}))?;
        check!(
            status.len() <= MAX_STATUS_BYTES,
            "transcript status byte budget exhausted"
        );
        let added = response
            .len()
            .checked_add(status.len())
            .ok_or("transcript byte count overflow")?;
        self.admit(added)?;
        write_new(&self.root.join(format!("{stem}.response.bin")), response)?;
        write_new(&self.root.join(format!("{stem}.status.json")), &status)?;
        self.bytes += added;
        Ok(())
    }

    fn admit(&self, added: usize) -> TestResult {
        let total = self
            .bytes
            .checked_add(added)
            .ok_or("transcript byte count overflow")?;
        check!(
            total <= MAX_TRANSCRIPT_BYTES,
            "transcript byte budget exhausted"
        );
        Ok(())
    }
}

pub(super) fn write_new(path: &Path, bytes: &[u8]) -> TestResult {
    check!(
        bytes.len() <= codec::MAX_WIRE_BYTES,
        "evidence file byte budget exhausted"
    );
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub(super) fn append(bytes: &mut Vec<u8>, next: &[u8]) -> TestResult {
    let total = bytes
        .len()
        .checked_add(next.len())
        .ok_or("fixture wire length overflow")?;
    check!(
        total <= codec::MAX_WIRE_BYTES,
        "fixture wire byte budget exhausted"
    );
    bytes.extend_from_slice(next);
    Ok(())
}

pub(super) fn terminal(frames: &[codec::Frame]) -> TestResult<u16> {
    let last = frames
        .last()
        .ok_or("SDK response omitted terminal frame")?
        .kind;
    check!(
        matches!(last, 0x0001..=0x0003),
        "SDK terminal frame must be last"
    );
    check!(eq; frames.iter().filter(|frame| matches!(frame.kind, 0x0001..=0x0003)).count(), 1);
    let outputs = frames.iter().filter(|frame| frame.kind == 0x0401).count();
    check!(eq; outputs, usize::from(last == 0x0003));
    Ok(last)
}

pub(super) async fn exchange(
    client: &reqwest::Client,
    url: &str,
    request: Vec<u8>,
    transcript: &mut Option<Transcript>,
) -> TestResult<Vec<codec::Frame>> {
    let stem = transcript
        .as_mut()
        .map(|trace| trace.begin(&request))
        .transpose()?;
    let request = bytes::Bytes::from(request);
    let response = client
        .post(url)
        .header("content-type", "application/vnd.restate.invocation.v6")
        .body(request)
        .send()
        .await;
    let mut response = match response {
        Ok(response) => response,
        Err(error) => {
            if let (Some(trace), Some(stem)) = (transcript.as_mut(), stem.as_ref()) {
                trace.finish(stem, None, &[], Some(error.to_string()))?;
            }
            return Err(error.into());
        }
    };
    let status = response.status();
    let mut bytes = Vec::new();
    let read = read_response(&mut response, &mut bytes).await;
    if let (Some(trace), Some(stem)) = (transcript, stem) {
        trace.finish(
            &stem,
            Some(status.as_u16()),
            &bytes,
            read.as_ref().err().map(ToString::to_string),
        )?;
    }
    read?;
    check!(eq; status, reqwest::StatusCode::OK);
    codec::frames(&bytes)
}

async fn read_response(response: &mut reqwest::Response, bytes: &mut Vec<u8>) -> TestResult {
    if let Some(length) = response.content_length() {
        check!(
            length <= u64::try_from(codec::MAX_WIRE_BYTES)?,
            "fixture response byte budget exhausted"
        );
    }
    while let Some(chunk) = response.chunk().await? {
        append(bytes, &chunk)?;
    }
    Ok(())
}
