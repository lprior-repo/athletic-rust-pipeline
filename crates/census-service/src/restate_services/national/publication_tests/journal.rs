use super::codec::{self, Frame};
use super::{fixtures, transport, TestResult};
use crate::restate_services::wire::NationalRequest;
use census_reconcile::identity::WorkflowIdentity;

mod tests;

pub struct Journal {
    pub original: NationalRequest,
    pub address_digest: String,
    pub identity: String,
    pub invocation: String,
    pub entries: Vec<Frame>,
    pub calls: Vec<Frame>,
    pub output: Option<serde_json::Value>,
    pub(super) ended: bool,
    pub held: Option<Frame>,
    pub hold_stage: Option<String>,
    pub partial_contacts: bool,
    pub unfinished_rosters: bool,
    pub seal_refusals: usize,
    pub source_store: Option<std::sync::Arc<census_store::Store>>,
    pub(super) transcript: Option<transport::Transcript>,
    exchanges: usize,
}

impl Journal {
    pub fn new(request: &NationalRequest) -> TestResult<Self> {
        Ok(Self {
            original: request.clone(),
            address_digest: "fixture-directory-digest".to_owned(),
            identity: WorkflowIdentity::national(
                request.season,
                request.revision,
                &request.jurisdictions,
            )
            .as_str()
            .to_owned(),
            invocation: "invocation-publication-fixture".to_owned(),
            entries: vec![Frame::new(
                0x0400,
                codec::blob(14, &codec::value(&serde_json::to_vec(request)?)?)?,
            )],
            calls: Vec::new(),
            output: None,
            ended: false,
            held: None,
            hold_stage: None,
            partial_contacts: false,
            unfinished_rosters: false,
            seal_refusals: 0,
            source_store: None,
            transcript: None,
            exchanges: 0,
        })
    }

    pub fn request_body(&self) -> TestResult<Vec<u8>> {
        let mut start = codec::blob(1, self.invocation.as_bytes())?;
        start.extend(codec::blob(2, self.invocation.as_bytes())?);
        start.extend(codec::integer(3, u64::try_from(self.entries.len())?));
        start.extend(codec::blob(6, self.identity.as_bytes())?);
        let mut bytes = Frame::new(0, start).encode()?;
        for entry in &self.entries {
            transport::append(&mut bytes, &entry.encode()?)?;
        }
        Ok(bytes)
    }

    pub async fn step(&mut self, client: &reqwest::Client, url: &str) -> TestResult<Vec<Frame>> {
        check!(
            self.exchanges < 256,
            "fixture total exchange budget exhausted"
        );
        self.exchanges += 1;
        let output =
            transport::exchange(client, url, self.request_body()?, &mut self.transcript).await?;
        transport::terminal(&output)?;
        for entry in &output {
            self.record(entry)?;
        }
        Ok(output)
    }

    pub async fn drive(&mut self, client: &reqwest::Client, url: &str) -> TestResult {
        for _ in 0..256 {
            self.step(client, url).await?;
            if self.output.is_some() || self.held.is_some() {
                return Ok(());
            }
        }
        Err("fixture national invocation did not settle within 256 resumptions".into())
    }

    pub fn release(&mut self) -> TestResult {
        let call = self.held.take().ok_or("no held publication stage")?;
        self.append(codec::notification(
            0x800D,
            call.number(11)?,
            &serde_json::to_vec(&fixtures::answer(&call, self)?)?,
        )?)?;
        self.hold_stage = None;
        Ok(())
    }

    pub fn release_refusal(&mut self) -> TestResult {
        let call = self.held.take().ok_or("no held seal measurement")?;
        let mut failure = codec::integer(1, 500);
        failure.extend(codec::blob(
            2,
            b"the census seal refused: jurisdiction sweeps remain owed",
        )?);
        let mut completion = codec::integer(1, call.number(11)?);
        completion.extend(codec::blob(6, &failure)?);
        self.append(Frame::new(0x800D, completion))?;
        self.seal_refusals = self
            .seal_refusals
            .checked_add(1)
            .ok_or("seal refusal count overflow")?;
        Ok(())
    }

    fn record(&mut self, entry: &Frame) -> TestResult {
        match entry.kind {
            0x040D => self.call(entry)?,
            0x040C => {
                self.append(entry.clone())?;
                let mut completion = codec::integer(1, entry.number(11)?);
                completion.extend(codec::blob(4, &[])?);
                self.append(Frame::new(0x800C, completion))?;
            }
            0x0005 => {
                self.append(codec::notification(
                    0x8011,
                    entry.number(1)?,
                    entry.bytes(14)?,
                )?)?;
            }
            0x0401 => {
                if let Some(failure) = codec::field(&entry.payload, 15)? {
                    let failure = Frame::new(0, failure.to_vec());
                    return Err(format!(
                        "national completed with terminal failure: {}",
                        failure.text(2)?
                    )
                    .into());
                }
                let value = codec::field(entry.bytes(14)?, 1)?.ok_or("missing national output")?;
                self.output = Some(serde_json::from_slice(value)?);
            }
            0x0002 => return Err(format!("SDK failed: {}", entry.text(2)?).into()),
            kind if (0x0400..0x8000).contains(&kind) => self.append(entry.clone())?,
            0x0001 => {}
            0x0003 => self.ended = true,
            kind => return Err(format!("unexpected SDK frame {kind:#x}").into()),
        }
        Ok(())
    }

    fn call(&mut self, call: &Frame) -> TestResult {
        check!(
            self.calls.len() < 512,
            "fixture child call budget exhausted"
        );
        self.append(call.clone())?;
        self.calls.push(call.clone());
        let mut invocation = codec::integer(1, call.number(10)?);
        invocation.extend(codec::blob(
            16,
            format!("child-{}", call.number(11)?).as_bytes(),
        )?);
        self.append(Frame::new(0x800E, invocation))?;
        if self.hold_stage.as_deref() == Some(call.text(2)?.as_str())
            || (self.hold_stage.as_deref() == Some(call.text(1)?.as_str())
                && (call.text(1)? != "Census" || call.text(2)? == "publication"))
        {
            self.held = Some(call.clone());
        } else {
            self.append(codec::notification(
                0x800D,
                call.number(11)?,
                &serde_json::to_vec(&fixtures::answer(call, self)?)?,
            )?)?;
        }
        Ok(())
    }

    fn append(&mut self, frame: Frame) -> TestResult {
        check!(
            self.entries.len() < codec::MAX_FRAMES - 1,
            "fixture journal frame budget exhausted"
        );
        let bytes = self.entries.iter().try_fold(
            frame.wire_size()?,
            |sum, entry| -> TestResult<usize> {
                sum.checked_add(entry.wire_size()?)
                    .ok_or_else(|| "fixture journal length overflow".into())
            },
        )?;
        check!(
            bytes <= codec::MAX_WIRE_BYTES,
            "fixture journal byte budget exhausted"
        );
        self.entries.push(frame);
        Ok(())
    }
}
