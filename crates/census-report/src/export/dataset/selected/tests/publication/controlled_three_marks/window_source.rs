use super::source::{capture, InstalledSource, TestResult};
use census_crawl::participation_evidence::recorded::record_independent_participation_window_packet;
use census_crawl::participation_evidence::{
    IndependentParticipationWindowPacketV1, ParticipationEvidenceRefV1,
};
use census_crawl::Recording;

pub(super) const SOURCE_URL: &str =
    "https://www.mhsaa.com/sports/boys-track-field/stories/controlled-full-window-2026";
pub(super) const SOURCE_BODY: &str = concat!(
    "<html><head><link rel=\"canonical\" href=\"https://www.mhsaa.com/sports/boys-track-field/stories/controlled-full-window-2026\"></head><body>",
    "<article class=\"node__article\" data-path=\"/sports/boys-track-field/stories/controlled-full-window-2026\" data-nid=\"9999100\">",
    "<p class=\"publish-date\">March 1, 2026</p>",
    "<h4><span class=\"text--secondary\"><a href=\"https://mi.milesplit.com/athletes/9999999-controlled-target\">Controlled Target</a> ♦ Detroit Catholic Central | Junior ♦ Track</span></h4>",
    "<p>SYNTHETIC CONTROLLED. No genuine-source, authenticated human approval or NCAA certification claim.</p>",
    "<p class=\"participation-window\">Full documented participation window | school year: 2025-26 | cohort: 2027 | from: 2026-03-01 | through: 2026-05-31 | documented summer boundary: 2026-05-31</p>",
    "</article></body></html>"
);

pub(super) fn install(installed: &InstalledSource) -> TestResult<ParticipationEvidenceRefV1> {
    let capture = capture(
        SOURCE_URL,
        SOURCE_BODY.as_bytes(),
        b"HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\n\r\n",
        "2026-10-16T00:13:00Z",
        "2026-10-16T00:13:01Z",
    )?;
    let packet = IndependentParticipationWindowPacketV1::new(capture)?;
    let reference = packet.reference()?;
    let recording = Recording::new();
    record_independent_participation_window_packet(&installed.store, &recording, &packet)?;
    census_crawl::recording::prepare_recorded(recording.drain(), "controlled-window-intake")?
        .apply(&installed.store)?;
    Ok(reference)
}
