use super::TestResult;
use census_store::{CaptureWriterV1, PublicResourceRootKindV1, Store};

#[path = "../../../../../../tests/common/controlled_public_capture.rs"]
pub(super) mod controlled_normalized_transport;

pub(super) fn install(
    store: &Store,
    writer: &CaptureWriterV1,
    site: census_domain::UsJurisdiction,
    directory: &[u8],
) -> TestResult {
    let owner = writer.public_resource_owner(store, site)?;
    for (kind, body, content_type, started_at, received_at, completed_at) in [
        (
            PublicResourceRootKindV1::Robots,
            b"User-agent: *\nAllow: /teams\n".as_slice(),
            "text/plain",
            "2026-10-02T12:00:00Z",
            "2026-10-02T12:00:01Z",
            "2026-10-02T12:00:02Z",
        ),
        (
            PublicResourceRootKindV1::MileSplitTeamDirectory,
            directory,
            "text/html; charset=utf-8",
            "2026-10-02T12:00:03Z",
            "2026-10-02T12:00:04Z",
            "2026-10-02T12:00:05Z",
        ),
    ] {
        let work = owner.for_root(store, kind)?;
        controlled_normalized_transport::retain_root(
            store,
            writer,
            &owner,
            work,
            controlled_normalized_transport::ControlledNormalizedCapture {
                body,
                content_type,
                started_at,
                received_at,
                completed_at,
            },
        )
        .map_err(|error| -> Box<dyn std::error::Error> { error })?;
    }
    Ok(())
}
