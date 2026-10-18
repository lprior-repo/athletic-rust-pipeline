use super::{capture, SourceTargetPacketV1, TestResult, PROFILE_URL, RESULTS_URL};

const HEADERS: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nX-Controlled-Fixture: three-marks-target\r\n\r\n";

const PROFILE: &str = r#"<!doctype html>
<html><head>
<link rel="canonical" href="https://mi.milesplit.com/athletes/9999999-controlled-target">
<script type="application/ld+json">{"@type":"Person","name":"Controlled Target","url":"https://mi.milesplit.com/athletes/9999999-controlled-target"}</script>
</head><body>
<p>SYNTHETIC CONTROLLED FIXTURE. Not a genuine athlete record.</p>
<h1 id="athleteName">Controlled Target</h1>
<p class="athlete-info">
<span class="current-school"><a href="https://mi.milesplit.com/teams/12439-detroit-catholic-central">Detroit Catholic Central</a></span>
<span class="grad-year">Class of 2027</span>
<span class="city-state">Detroit, MI</span>
</p></body></html>"#;

const LINKED: &str = r#"<!doctype html>
<html><head>
<link rel="canonical" href="https://mi.milesplit.com/meets/999999-controlled-championships/teams/12439">
<script type="application/ld+json">{"@type":"SportsEvent","name":"Controlled Championships","startDate":"2026-02-28","endDate":"2026-02-28","sport":"Outdoor Track & Field","location":{"address":{"addressRegion":"MI"}},"url":"https://mi.milesplit.com/meets/999999-controlled-championships/teams/12439"}</script>
</head><body>
<p>SYNTHETIC CONTROLLED FIXTURE. Not a genuine competition result.</p>
<h2><a href="https://mi.milesplit.com/teams/12439-detroit-catholic-central">Detroit Catholic Central</a> Results</h2>
<table><tbody>
<tr data-team-row data-search="Controlled Target" data-gender="m" data-event="42">
<td class="meetTeams__participant"><a href="https://mi.milesplit.com/athletes/9999999-controlled-target">Controlled Target</a></td>
<td><span class="meetTeams__markValue">49.00</span></td>
<td class="meetTeams__source"><a href="https://mi.milesplit.com/meets/999999-controlled-championships/results/9999990/auto">Results</a></td>
</tr></tbody></table>
</body></html>"#;

pub(super) fn packet() -> TestResult<SourceTargetPacketV1> {
    let profile = capture(
        PROFILE_URL,
        PROFILE.as_bytes(),
        HEADERS,
        "2026-10-16T00:50:21Z",
        "2026-10-16T00:50:27Z",
    )?;
    let linked = capture(
        RESULTS_URL,
        LINKED.as_bytes(),
        HEADERS,
        "2026-10-16T00:49:13Z",
        "2026-10-16T00:49:19Z",
    )?;
    Ok(SourceTargetPacketV1::new(profile, linked)?)
}
