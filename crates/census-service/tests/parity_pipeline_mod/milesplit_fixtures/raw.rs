use anyhow::{Context, Result};
use census_crawl::milesplit;
use census_domain::model::{
    EventKind, ExactSeconds, Gender, Mark, SchoolYear, Sport, TimingMethod,
};

use super::TROY_URL;

pub(super) fn validate_projection(body: &str, boys: bool) -> Result<()> {
    let rsid = if boys { "1266815" } else { "1266814" };
    let url = format!("{TROY_URL}/{rsid}/raw");
    check!(body.contains(&format!("<link rel=\"canonical\" href=\"{url}\" />")));
    let pre = body
        .split_once("<pre>")
        .context("projection has no raw block")?
        .1;
    let block = pre
        .split_once("</pre>")
        .context("projection has no closing tag")?
        .0;
    let lines: Vec<_> = block.split_inclusive('\n').collect();
    check!(eq; lines.len(), 8);
    check!(lines.iter().all(|line| line.ends_with("\r\n")));
    let page = milesplit::parse_raw(body, &url)?;
    check!(eq; page.meet.name, "Troy Invitational #2");
    check!(eq; page.meet.date, "2026-03-27");
    check!(eq; page.meet.end_date.as_deref(), Some("2026-03-27"));
    check!(eq; page.sport, Some(Sport::OutdoorTrack));
    check!(eq; page.region.as_deref(), Some("AL"));
    check!(eq; page.school_year,
    SchoolYear::new(2025).context("2025 school year")?);
    check!(eq; page.meet.rows_parsed, 4);
    check!(eq; page.meet.rows_skipped, 0);
    check!(eq; page.meet.events.len(), 1);
    validate_projected_rows(&page, boys)?;
    validate_grade_locator(body, &page, boys)
}

fn validate_projected_rows(page: &milesplit::RawPage, boys: bool) -> Result<()> {
    let event = page
        .meet
        .events
        .first()
        .context("projection has no event")?;
    check!(eq; event.kind, EventKind::Track100m);
    check!(eq; event.gender,
    if boys { Gender::Boys } else { Gender::Girls });
    check!(eq; event.round.as_deref(), Some("Finals"));
    let actual: Vec<_> = event
        .rows
        .iter()
        .map(|row| {
            (
                row.name.as_str(),
                row.school.as_str(),
                row.grade.map(|grade| grade.get()),
                row.mark.clone(),
                row.timing,
            )
        })
        .collect();
    let expected: Vec<_> = expected_rows(boys)
        .into_iter()
        .map(|(name, school, grade, mark)| -> Result<_> {
            Ok((
                name,
                school,
                grade,
                Mark::TimeSeconds(ExactSeconds::from_parts(
                    i64::from(mark)
                        .checked_mul(10_000_000)
                        .context("fixture time overflow")?,
                    2,
                )?),
                Some(TimingMethod::Fat),
            ))
        })
        .collect::<Result<_>>()?;
    check!(eq; actual, expected);
    Ok(())
}

type ExpectedRow = (&'static str, &'static str, Option<u8>, i32);

fn expected_rows(boys: bool) -> [ExpectedRow; 4] {
    if boys {
        [
            ("Braylin Jackson", "Pike County", Some(12), 1093),
            ("Briece Baker", "Charles Henderson", Some(9), 1095),
            ("Kourtni Christian", "Charles Henderson", Some(10), 1106),
            (
                "Carmyree Pearson-White",
                "Carver Montgomery",
                Some(12),
                1106,
            ),
        ]
    } else {
        [
            ("Payton Ousley", "Charles Henderson", Some(12), 1240),
            ("Joy Taylor", "Brewbaker Tech", Some(11), 1251),
            ("Shai'Vonne Grace", "Charles Henderson", None, 1289),
            ("Laniya Wheeler", "Pike County", Some(10), 1302),
        ]
    }
}

fn validate_grade_locator(body: &str, page: &milesplit::RawPage, boys: bool) -> Result<()> {
    check!(eq; page.grade_issues.len(), if boys { 0 } else { 1 });
    if boys {
        return Ok(());
    }
    let issue = page
        .grade_issues
        .first()
        .context("eighth-grade row has no issue")?;
    check!(eq; issue.kind, milesplit::RawGradeIssueKind::OutsideHighSchool);
    check!(eq; issue.raw_token, "8");
    check!(eq; issue.row.ordinal, 7);
    let start = body
        .find("   3 Shai'Vonne Grace")
        .context("original eighth-grade row missing")?;
    let row = body
        .get(start..)
        .and_then(|tail| tail.split("\r\n").next())
        .context("original row has no line")?;
    check!(eq; issue.row.byte_offset, start);
    check!(eq; issue.row.byte_length, row.len());
    Ok(())
}

pub(super) fn validate_capture(file: &str, body: &str) -> Result<()> {
    let (url, name, date, region, sport, year) = capture_facts(file)?;
    let page = milesplit::parse_raw(body, url)?;
    check!(eq; page.meet.name, name);
    check!(eq; page.meet.date, date);
    check!(eq; page.region.as_deref(), Some(region));
    check!(eq; page.sport, Some(sport));
    check!(eq; page.school_year,
    SchoolYear::new(year).context("captured school year")?);
    Ok(())
}

type CaptureFacts = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Sport,
    i16,
);

fn capture_facts(file: &str) -> Result<CaptureFacts> {
    Ok(match file {
        "oh_meet_770621_rs1321880_raw.html" => (
            "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/raw",
            "Beaver Eastern Invite",
            "2026-09-19",
            "OH",
            Sport::CrossCountry,
            2026,
        ),
        "nc_meet_684812_rs1283641_raw.html" => (
            "https://nc.milesplit.com/meets/684812-asics-carolina-distance-carnival-2026/results/1283641/raw",
            "ASICS Carolina Distance Carnival",
            "2026-04-17",
            "NC",
            Sport::OutdoorTrack,
            2025,
        ),
        "dc_meet_735841_results_legacy.html" => (
            "https://www.milesplit.com/meets/735841-stancs-home-meet-1-2026/results/1257095/raw",
            "STA/NCS Home Meet #1",
            "2026-03-11",
            "DC",
            Sport::OutdoorTrack,
            2025,
        ),
        "dc_meet_764735_results_inline.html" => (
            "https://www.milesplit.com/meets/764735-dc10-track-fest-hosted-by-light-horse-track-club-2026/results",
            "DC10 Track Fest Hosted by Light Horse Track Club",
            "2026-03-28",
            "DC",
            Sport::OutdoorTrack,
            2025,
        ),
        _ => anyhow::bail!("unsupported raw fixture {file}"),
    })
}
