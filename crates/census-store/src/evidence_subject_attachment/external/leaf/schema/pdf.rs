use super::{parse, Error, StoreResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use serde::Deserialize;
use serde_json::value::RawValue;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet<'a> {
    version: u16,
    interpretation: &'a str,
    #[serde(borrow)]
    capture: &'a RawValue,
}

pub(super) fn capture<'a>(
    raw: &'a RawValue,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<&'a RawValue> {
    let packet: Packet<'a> = parse(raw, grant)?;
    if packet.version != 1 || packet.interpretation != "mhsaa_approved_coop_pdf_v1" {
        return Err(Error::Corrupt("MHSAA PDF root interpretation").into());
    }
    Ok(packet.capture)
}
