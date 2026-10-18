use super::source::{capture, CurrentSchoolInput, InstalledSource, TestResult};
use census_crawl::participation_evidence::provider_school::ProviderSchoolPacketV1;
use census_crawl::participation_evidence::recorded::record_provider_school_intake;
use census_crawl::Recording;

const CO_DIRECTORY_URL: &str = "https://co.milesplit.com/teams";
const CO_TEAM_URL: &str = "https://co.milesplit.com/teams/9999000-controlled-colorado-high-school";
const CO_TEAM_NAME: &str = "Controlled Colorado High School";
const CO_TEAM_CITY_STATE: &str = "Denver, CO, USA";
const CO_ATHLETE_URL: &str = "https://co.milesplit.com/athletes/9999999-controlled-target";
const CO_ATHLETE_ROSTER_NAME: &str = "Target, Controlled";
const CO_GRAD_YEAR: &str = "2027";
const DIRECTORY_HEADERS: &[u8] =
    b"HTTP/2 200\r\ndate: Fri, 16 Oct 2026 02:58:00 GMT\r\ncontent-type: text/html; charset=UTF-8\r\nserver: nginx\r\n\r\n";
const ROSTER_HEADERS: &[u8] =
    b"HTTP/2 200\r\ndate: Fri, 16 Oct 2026 03:00:01 GMT\r\ncontent-type: text/html; charset=UTF-8\r\nserver: nginx\r\n\r\n";

fn roster_url() -> String {
    format!("{}/roster", CO_TEAM_URL)
}

fn directory_html() -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="application-name" content="MileSplit Colorado (SYNTHETIC CONTROLLED)">
<link rel="canonical" href="{directory}">
</head>
<body>
<p>SYNTHETIC CONTROLLED directory fixture for the 2026-27 Colorado cohort; not a genuine MileSplit capture.</p>
<table>
<tr><td><a href="{team}">{name}</a></td><td>{city}</td></tr>
</table>
</body>
</html>"#,
        directory = CO_DIRECTORY_URL,
        team = CO_TEAM_URL,
        name = CO_TEAM_NAME,
        city = CO_TEAM_CITY_STATE,
    )
}

fn roster_html(roster_url: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="application-name" content="MileSplit Colorado (SYNTHETIC CONTROLLED)">
<link rel="canonical" href="{roster}">
</head>
<body>
<section>
<h4>Current Roster</h4>
<p>SYNTHETIC CONTROLLED roster fixture for the 2026-27 Colorado cohort; not a genuine MileSplit capture.</p>
<ul id="rosterDataset">
<li class="athlete-row data-row"><div class="data-point w-30 w-md-50 d-flex align-items-center"><a href="{athlete}">{name}</a></div><div class="data-point w-20 w-md-10 text-lighter text-center text-uppercase column-gender">m</div><div class="data-point w-20 text-lighter text-center column-grad-year">{grad}</div><div class="data-point w-20 w-md-10 text-lighter text-center" data-season-id="0"><svg class="icon icon-no"></svg></div><div class="data-point w-10 text-lighter text-center" data-season-id="2"><svg class="icon icon-yes"></svg></div><div class="data-point w-10 text-lighter text-center" data-season-id="3"><svg class="icon icon-no"></svg></div></li>
</ul>
</section>
</body>
</html>"#,
        roster = roster_url,
        athlete = CO_ATHLETE_URL,
        name = CO_ATHLETE_ROSTER_NAME,
        grad = CO_GRAD_YEAR,
    )
}

pub(super) fn install(installed: &InstalledSource) -> TestResult<CurrentSchoolInput> {
    let directory_body = directory_html();
    let roster_url = roster_url();
    let roster_body = roster_html(&roster_url);
    let directory = capture(
        CO_DIRECTORY_URL,
        directory_body.as_bytes(),
        DIRECTORY_HEADERS,
        "2026-10-16T02:57:54Z",
        "2026-10-16T02:58:00Z",
    )?;
    let roster = capture(
        &roster_url,
        roster_body.as_bytes(),
        ROSTER_HEADERS,
        "2026-10-16T03:00:00Z",
        "2026-10-16T03:00:01Z",
    )?;
    let packet = ProviderSchoolPacketV1::new(directory, roster)?;
    let reference = packet.reference()?;
    let recording = Recording::new();
    let school = record_provider_school_intake(&installed.store, &recording, &packet)?;
    census_crawl::participation_evidence::recorded::record_provider_roster_intake(
        &installed.store,
        &recording,
        &packet,
    )?;
    match census_crawl::recording::prepare_recorded(
        recording.drain(),
        "controlled-colorado-school-intake",
    )?
    .apply(&installed.store)?
    {
        census_store::Application::Written(receipt) => {
            drop(receipt);
        }
        other => {
            return Err(format!(
                "controlled colorado school intake did not write a new receipt: {other:?}"
            )
            .into());
        }
    }
    Ok(CurrentSchoolInput { school, reference })
}
