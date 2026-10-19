use super::super::calendar_source;
use super::super::documentary_material;
use super::super::documentary_review;
use super::super::source::{self, InstalledSource, TestResult};
use super::install_reviewed;
use census_crawl::participation_evidence::recorded::record_independent_participation_packet;
use census_crawl::participation_evidence::{
    IndependentParticipationPacketV1, ParticipationEvidenceRefV1,
};
use census_crawl::Recording;
use census_domain::evidence_subject_attachment::{
    AttachmentPredecessorV1, AttachmentReplayGrantV1,
};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::ReplayLimits;
use census_domain::result_calendar_correspondence::{
    ResultCalendarActionV1, ResultCalendarPredecessorV1, ResultCalendarRecordReferenceV1,
    ResultCalendarReviewCommandV1,
};
use census_store::{Receipt, Store};

const LARGE_PACKET_COUNT: usize = 4;
const REVIEW_REVISION_COUNT: usize = 8;
const SCHEDULE_PADDING_BYTES: usize = 500 * 1024;
const SCHEDULE_URL: &str = "https://dccshamrocks.net/sports/track-and-field/schedule/2025-26";
const SCHEDULE_HEADERS: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nX-Controlled-Fixture: three-marks-size-bound\r\n\r\n";
const STATUS_HEADERS: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nX-Controlled-Fixture: size-bound-independent-participation\r\n\r\n";

fn padded_schedule(
    index: usize,
) -> TestResult<census_crawl::participation_evidence::RetainedHtmlCaptureV1> {
    let base = r#"<!doctype html><html><head>
<link rel="canonical" href="https://dccshamrocks.net/sports/track-and-field/schedule/2025-26">
<meta name="controlled-test-fixture" content="three-marks-schedule-v1">
</head><body><p>SYNTHETIC CONTROLLED SCHEDULE. Not a fetched official page or independent genuine evidence.</p>
<h2>2025-26 Track and Field Schedule</h2><ul>
<li class="sidearm-schedule-game" data-game-id="990001" data-sport-id="15"><div class="sidearm-schedule-game-opponent-date"><span>Feb 28 (Sat)</span></div><div class="sidearm-schedule-game-opponent-name">Controlled Track Meet 495051</div><div class="sidearm-schedule-game-location">Controlled Track Venue</div></li>
<li class="sidearm-schedule-game" data-game-id="990002" data-sport-id="15"><div class="sidearm-schedule-game-opponent-date"><span>May 15 (Fri)</span></div><div class="sidearm-schedule-game-opponent-name">Controlled Track Meet 502100</div><div class="sidearm-schedule-game-location">Controlled Track Venue</div></li>
<li class="sidearm-schedule-game" data-game-id="990003" data-sport-id="15"><div class="sidearm-schedule-game-opponent-date"><span>May 16 (Sat)</span></div><div class="sidearm-schedule-game-opponent-name">Controlled Track Meet 502101</div><div class="sidearm-schedule-game-location">Controlled Track Venue</div></li>
</ul></body></html>"#;
    let body = format!(
        "{base}\n<!-- controlled padding {index} {} -->",
        "x".repeat(SCHEDULE_PADDING_BYTES)
    );
    source::capture(
        SCHEDULE_URL,
        body.as_bytes(),
        SCHEDULE_HEADERS,
        "2026-10-16T00:11:00Z",
        "2026-10-16T00:11:01Z",
    )
}

fn padded_status(
    index: usize,
) -> TestResult<census_crawl::participation_evidence::RetainedHtmlCaptureV1> {
    let url = format!(
        "https://www.mhsaa.com/sports/boys-track-field/stories/controlled-size-point-{index}"
    );
    let path = format!("/sports/boys-track-field/stories/controlled-size-point-{index}");
    let body = format!(
        "<!doctype html><html><head><link rel=\"canonical\" href=\"{url}\"></head><body>\
         <article class=\"node__article\" data-path=\"{path}\" data-nid=\"2000{index}\">\
         <p class=\"publish-date\">May 15, 2026</p>\
         <h4><span class=\"text--secondary\">Controlled Target ♦ Detroit Catholic Central | Junior ♦ Track</span></h4>\
         <p>SYNTHETIC CONTROLLED FIXTURE. No genuine publication or authority. Point 50.00 recorded on 2026-05-15.</p>\
         <p>Corroborates provider profile at https://mi.milesplit.com/athletes/9999999-controlled-target for independent review.</p>\
         </article></body></html>"
    );
    source::capture(
        &url,
        body.as_bytes(),
        STATUS_HEADERS,
        "2026-10-16T00:12:00Z",
        "2026-10-16T00:12:01Z",
    )
}

fn install_padded_packets(
    installed: &InstalledSource,
) -> TestResult<Vec<ParticipationEvidenceRefV1>> {
    let mut references = Vec::new();
    references.try_reserve_exact(LARGE_PACKET_COUNT)?;
    for index in 0..LARGE_PACKET_COUNT {
        let packet =
            IndependentParticipationPacketV1::new(padded_status(index)?, padded_schedule(index)?)?;
        let recording = Recording::new();
        record_independent_participation_packet(&installed.store, &recording, &packet)?;
        census_crawl::recording::prepare_recorded(
            recording.drain(),
            &format!("controlled-size-bound-independent-intake-{index}"),
        )?
        .apply(&installed.store)?;
        references.push(packet.reference()?);
    }
    Ok(references)
}

fn accept_padded_sources(
    installed: &InstalledSource,
    references: &[ParticipationEvidenceRefV1],
) -> TestResult<
    Vec<
        census_crawl::participation_evidence::result_ownership::calendar_correspondence::ResultCalendarSourceProposalV1,
    >,
> {
    let mut proposals = Vec::new();
    proposals.try_reserve_exact(references.len())?;
    for (index, reference) in references.iter().enumerate() {
        documentary_review::accept(
            installed,
            reference,
            documentary_material::Kind::Point,
            AttachmentPredecessorV1::First,
            &format!("controlled-size-bound-source-{index}"),
        )
        .map_err(|error| format!("accept padded documentary {index}: {error}"))?;
    }
    for (index, reference) in references.iter().enumerate() {
        proposals.push(
            calendar_source::propose(installed, reference)
                .map_err(|error| format!("propose padded calendar {index}: {error}"))?,
        );
    }
    Ok(proposals)
}

fn freeze_calendar(
    store: &Store,
) -> TestResult<(
    Vec<u8>,
    census_domain::evidence_subject_attachment::AttachmentObjectReferenceV1,
    u64,
    StoreIdentity,
)> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let current =
        snapshot.read_current_result_calendar_correspondences_v1(&attachments, &mut grant)?;
    let frozen = snapshot.freeze_public_identity_v2()?;
    let proof = current.freeze_closure(&frozen, &mut grant)?;
    let sequence = current.snapshot_sequence();
    let identity = current.store_identity().clone();
    let (reference, bytes) = proof.into_parts();
    Ok((bytes, reference, sequence, identity))
}

#[test]
fn calendar_large_review_history_replays_exact_source_decisions() -> TestResult {
    let (installed, _points, _initial) = install_reviewed()?;
    revocation::enroll_calendar(
        &installed.store,
        "controlled-calendar-enroll",
        "Explicit controlled calendar enrollment after school9 and reviewed status",
    )?;
    let references = install_padded_packets(&installed)
        .map_err(|error| format!("install padded packet batch: {error}"))?;
    let proposals = accept_padded_sources(&installed, &references)
        .map_err(|error| format!("accept padded source batch: {error}"))?;
    let mut previous: Option<(ResultCalendarReviewCommandV1, Receipt)> = None;
    for (index, proposal) in proposals
        .iter()
        .cycle()
        .take(REVIEW_REVISION_COUNT)
        .enumerate()
    {
        let action = match previous.as_ref() {
            None => ResultCalendarActionV1::Accept {
                prior: ResultCalendarPredecessorV1::First,
            },
            Some((command, receipt)) => {
                let digest =
                    census_domain::result_calendar_correspondence::ResultCalendarDigestV1::parse(
                        receipt.digest.as_str(),
                    )?;
                ResultCalendarActionV1::Accept {
                    prior: ResultCalendarPredecessorV1::After(ResultCalendarRecordReferenceV1 {
                        operation: command.operation.clone(),
                        digest,
                    }),
                }
            }
        };
        previous = Some(
            revocation::apply_calendar_action(
                &installed.store,
                proposal,
                &format!("controlled-size-bound-calendar-{index}"),
                action,
                "Explicit controlled calendar review",
            )
            .map_err(|error| format!("apply padded calendar action {index}: {error}"))?,
        );
    }
    let (bytes, reference, sequence, identity) = freeze_calendar(&installed.store)?;
    check!(bytes.len() <= ReplayLimits::SMALL.total_text_bytes);
    let mut grant = AttachmentReplayGrantV1::new();
    let replayed = census_store::replay_result_calendar_source_proof_v1(
        &bytes, &reference, &identity, sequence, &mut grant,
    )?;
    check!(eq; replayed.records().len(), REVIEW_REVISION_COUNT);
    let last = previous.ok_or("calendar history is empty")?;
    let active = replayed.active_records(&mut grant)?.collect::<Vec<_>>();
    check!(eq; active.len(), 1);
    let active = active
        .first()
        .ok_or("calendar history has no active decision")?;
    check!(eq; active.command, last.0);
    Ok(())
}