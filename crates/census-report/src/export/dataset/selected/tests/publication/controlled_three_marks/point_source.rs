use super::source::{capture, InstalledSource, TestResult, MARKS};
use census_crawl::participation_evidence::recorded::record_independent_participation_packet;
use census_crawl::participation_evidence::{
    IndependentParticipationPacketV1, ParticipationEvidenceRefV1,
};
use census_crawl::Recording;

fn status_capture(
    index: usize,
    mark: &super::source::ControlledMark,
) -> TestResult<census_crawl::participation_evidence::RetainedHtmlCaptureV1> {
    let url =
        format!("https://www.mhsaa.com/sports/boys-track-field/stories/controlled-point-{index}");
    let path = format!("/sports/boys-track-field/stories/controlled-point-{index}");
    let day = mark.date;
    let date_display = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d")?
        .format("%B %-d, %Y")
        .to_string();
    let whole = mark.time / 100;
    let fraction = mark.time % 100;
    let body = format!(
        "<!doctype html><html><head><link rel=\"canonical\" href=\"{url}\"></head><body>\
        <article class=\"node__article\" data-path=\"{path}\" data-nid=\"1000{index}\">\
        <p class=\"publish-date\">{date_display}</p>\
        <h4><span class=\"text--secondary\">Controlled Target ♦ Detroit Catholic Central | Junior ♦ Track</span></h4>\
        <p>SYNTHETIC CONTROLLED FIXTURE. No genuine publication or authority. Point {whole}.{fraction:02} recorded on {day}.</p>\
        <p>Corroborates provider profile at https://mi.milesplit.com/athletes/9999999-controlled-target for independent review.</p>\
        </article></body></html>"
    );
    capture(
        &url,
        body.as_bytes(),
        b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nX-Controlled-Fixture: independent-participation\r\n\r\n",
        "2026-10-16T00:12:00Z",
        "2026-10-16T00:12:01Z",
    )
}

fn schedule_capture() -> TestResult<census_crawl::participation_evidence::RetainedHtmlCaptureV1> {
    let url = "https://dccshamrocks.net/sports/track-and-field/schedule/2025-26";
    let body = r#"<!doctype html><html><head>
<link rel="canonical" href="https://dccshamrocks.net/sports/track-and-field/schedule/2025-26">
<meta name="controlled-test-fixture" content="three-marks-schedule-v1">
</head><body><p>SYNTHETIC CONTROLLED SCHEDULE. Not a fetched official page or independent genuine evidence.</p>
<h2>2025-26 Track and Field Schedule</h2><ul>
<li class="sidearm-schedule-game" data-game-id="990001" data-sport-id="15">
<div class="sidearm-schedule-game-opponent-date"><span>Feb 28 (Sat)</span></div>
<div class="sidearm-schedule-game-opponent-name">Controlled Track Meet 495051</div>
<div class="sidearm-schedule-game-location">Controlled Track Venue</div>
</li>
<li class="sidearm-schedule-game" data-game-id="990002" data-sport-id="15">
<div class="sidearm-schedule-game-opponent-date"><span>May 15 (Fri)</span></div>
<div class="sidearm-schedule-game-opponent-name">Controlled Track Meet 502100</div>
<div class="sidearm-schedule-game-location">Controlled Track Venue</div>
</li>
<li class="sidearm-schedule-game" data-game-id="990003" data-sport-id="15">
<div class="sidearm-schedule-game-opponent-date"><span>May 16 (Sat)</span></div>
<div class="sidearm-schedule-game-opponent-name">Controlled Track Meet 502101</div>
<div class="sidearm-schedule-game-location">Controlled Track Venue</div>
</li>
</ul></body></html>"#;
    capture(
        url,
        body.as_bytes(),
        b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nX-Controlled-Fixture: three-marks-schedule-v1\r\n\r\n",
        "2026-10-16T00:11:00Z",
        "2026-10-16T00:11:01Z",
    )
}

pub(super) fn install(installed: &InstalledSource) -> TestResult<Vec<ParticipationEvidenceRefV1>> {
    let schedule = schedule_capture()?;
    let mut refs = Vec::new();
    refs.try_reserve_exact(MARKS.len())?;

    for (index, mark) in MARKS.iter().enumerate() {
        let status = status_capture(index, mark)?;
        let packet = IndependentParticipationPacketV1::new(status, schedule.clone())?;
        let recording = Recording::new();
        record_independent_participation_packet(&installed.store, &recording, &packet)?;
        let prepared = census_crawl::recording::prepare_recorded(
            recording.drain(),
            &format!("controlled-independent-intake-{index}"),
        )?;
        prepared.apply(&installed.store)?;
        refs.push(packet.reference()?);
    }

    Ok(refs)
}
