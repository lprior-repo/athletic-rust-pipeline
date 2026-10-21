use super::{codec, key, ledger, transport};
use crate::restate_services::wire::{NationalRequestBinding, TeamsSourceRequest};
use census_domain::model::Revision;
use census_reconcile::identity::WorkflowIdentity;
use census_store::{CaptureWriterV1, Store};
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Default)]
struct Events {
    output: Option<Vec<u8>>,
    failure: Option<String>,
}

pub(super) struct Driver {
    pub(super) writer: Arc<CaptureWriterV1>,
    journal: Vec<codec::Frame>,
    pub(super) run_values: Vec<serde_json::Value>,
}

impl Driver {
    pub(super) fn new(request: &impl Serialize, writer: &Arc<CaptureWriterV1>) -> TestResult<Self> {
        Ok(Self {
            writer: Arc::clone(writer),
            journal: vec![codec::Frame::new(
                0x0400,
                codec::blob(14, &codec::value(&serde_json::to_vec(request)?)?)?,
            )],
            run_values: Vec::new(),
        })
    }

    async fn step(
        &mut self,
        store: &Arc<Store>,
        request: &TeamsSourceRequest,
        handler: &str,
    ) -> TestResult<Events> {
        let frames =
            transport::exchange(store, request, &self.writer, handler, &self.journal).await?;
        let mut events = Events::default();
        for frame in frames {
            match frame.kind {
                0x040D => self.record_call(store, &frame)?,
                0x0411 => self.journal.push(frame),
                0x0005 => self.record_run(&frame)?,
                0x0401 => {
                    if let Some(value) = codec::field(&frame.payload, 14)? {
                        events.output = Some(
                            codec::field(value, 1)?
                                .ok_or("missing output content")?
                                .to_vec(),
                        );
                    } else {
                        events.failure = Some(failure(frame.bytes(15)?)?);
                    }
                }
                0x0002 => events.failure = Some(frame.text(2)?),
                0x0001 | 0x0003 => {}
                _ => {
                    return Err(
                        format!("unexpected origin continuation SDK frame {frame:?}").into(),
                    )
                }
            }
        }
        check!(self.journal.len() < codec::MAX_FRAMES);
        Ok(events)
    }

    fn record_call(&mut self, store: &Store, call: &codec::Frame) -> TestResult {
        check!(eq; call.text(1)?, "NationalCensus");
        check!(eq; call.text(2)?, "request");
        check!(codec::field(&call.payload, 3)?.is_none_or(<[u8]>::is_empty));
        let manifest = store
            .run_manifest()?
            .ok_or("continuation fixture run absent")?;
        let key = WorkflowIdentity::national(
            manifest.run.season(),
            Revision(manifest.run.revision()),
            &manifest.jurisdictions,
        );
        check!(eq; call.text(5)?, key.as_str());
        self.journal.push(call.clone());
        let mut invocation = codec::integer(1, call.number(10)?);
        invocation.extend(codec::blob(
            16,
            format!("national-request-{}", call.number(11)?).as_bytes(),
        )?);
        self.journal.push(codec::Frame::new(0x800E, invocation));
        let response = NationalRequestBinding::Bound(Box::new(transport::national_request(store)?));
        self.journal.push(codec::notification(
            0x800D,
            call.number(11)?,
            &serde_json::to_vec(&response)?,
        )?);
        Ok(())
    }

    fn record_run(&mut self, frame: &codec::Frame) -> TestResult {
        let completion = if let Some(value) = codec::field(&frame.payload, 14)? {
            self.run_values.push(serde_json::from_slice(value)?);
            codec::notification(0x8011, frame.number(1)?, value)?
        } else {
            let mut payload = codec::integer(1, frame.number(1)?);
            payload.extend(codec::blob(6, frame.bytes(15)?)?);
            codec::Frame::new(0x8011, payload)
        };
        self.journal.push(completion);
        Ok(())
    }

    pub(super) async fn until_committed(
        &mut self,
        store: &Arc<Store>,
        request: &TeamsSourceRequest,
    ) -> TestResult {
        for _ in 0..12 {
            let events = self.step(store, request, "continue_origin_held").await?;
            check!(events.output.is_none() && events.failure.is_none());
            if store.journal_contains(
                ledger::origin_continuation::PHASE,
                &format!("{}/attempt/2/outcome", key(request)),
            )? {
                return Ok(());
            }
        }
        Err("continuation did not commit attempt two within twelve SDK resumptions".into())
    }

    pub(super) async fn until_output(
        &mut self,
        store: &Arc<Store>,
        request: &TeamsSourceRequest,
        handler: &str,
    ) -> TestResult<Vec<u8>> {
        for _ in 0..12 {
            let events = self.step(store, request, handler).await?;
            check!(events.failure.is_none());
            if let Some(output) = events.output {
                return Ok(output);
            }
        }
        Err("missing continuation output within twelve SDK resumptions".into())
    }

    pub(super) async fn until_failure(
        &mut self,
        store: &Arc<Store>,
        request: &TeamsSourceRequest,
    ) -> TestResult<String> {
        for _ in 0..4 {
            let events = self.step(store, request, "continue_origin_held").await?;
            check!(events.output.is_none());
            if let Some(failure) = events.failure {
                return Ok(failure);
            }
        }
        Err("missing continuation refusal within four SDK resumptions".into())
    }

    pub(super) fn withhold_last_ack(&mut self) -> TestResult<serde_json::Value> {
        let position = self
            .journal
            .iter()
            .rposition(|frame| frame.kind == 0x8011)
            .ok_or("missing committed attempt Run completion")?;
        let withheld = self.journal.remove(position);
        let completion = withheld.number(1)?;
        let mut matching_command = false;
        for frame in self.journal.iter().filter(|frame| frame.kind == 0x0411) {
            matching_command |= frame.number(11)? == completion;
        }
        check!(matching_command, "withheld completion lost its Run command");
        let inner = codec::field(&withheld.payload, 5)?.ok_or("missing withheld Run value")?;
        let value = codec::field(inner, 1)?.ok_or("missing withheld Run content")?;
        Ok(serde_json::from_slice(value)?)
    }

    pub(super) fn persist(&self, path: &Path) -> TestResult {
        let mut bytes = Vec::new();
        for frame in &self.journal {
            bytes.extend(frame.encode()?);
        }
        check!(bytes.len() <= codec::MAX_WIRE_BYTES);
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub(super) fn restore(&mut self, path: &Path) -> TestResult {
        self.journal = codec::frames(&std::fs::read(path)?)?;
        Ok(())
    }
}

fn failure(bytes: &[u8]) -> TestResult<String> {
    let message = codec::field(bytes, 2)?.ok_or("missing terminal failure message")?;
    Ok(std::str::from_utf8(message)?.to_owned())
}
