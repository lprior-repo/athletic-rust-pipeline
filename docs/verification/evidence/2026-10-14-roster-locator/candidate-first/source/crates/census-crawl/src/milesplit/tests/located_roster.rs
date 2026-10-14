use super::*;

const PREFIX: &str = "é🏃<ul id=\"rosterDataset\">";
const OPEN: &str = "<li class=\"athlete-row data-row\">";
const CLOSE: &str = "</li>";
const INNER: &str = "<a href=\"https://wi.milesplit.com/athletes/42-example\">D&apos;Arc, José</a><div class=\"column-gender\">f</div><div class=\"column-grad-year\">2027</div><div class=\"data-point\" data-season-id=\"1\"><svg class=\"icon icon-yes\"></svg></div>";
const REJECTED: &str = "<a href=\"https://wi.milesplit.com/athletes/99-rejected\">Missing, Year</a><div class=\"column-grad-year\">invalid</div>";

fn team() -> TestResult<TeamRef> {
    parse_team_index(TEAMS)?
        .teams
        .into_iter()
        .next()
        .ok_or_else(|| "missing fixture team".into())
}

fn row(inner: &str) -> String {
    format!("{OPEN}{inner}{CLOSE}")
}

fn span(body: &str, locator: SourceRowLocator) -> TestResult<&str> {
    let end = locator
        .byte_offset
        .checked_add(locator.byte_length)
        .ok_or("locator overflow")?;
    body.get(locator.byte_offset..end)
        .ok_or_else(|| "locator outside UTF-8 body".into())
}

fn check_member(
    body: &str,
    member: &LocatedRosterAthlete,
    ordinal: u32,
    offset: usize,
    inner: &str,
) -> TestResult {
    check!(eq; member.locator(), SourceRowLocator {
        ordinal, byte_offset: offset, byte_length: inner.len(),
    });
    check!(eq; span(body, member.locator())?, inner);
    Ok(())
}

#[test]
fn positive_roster_locator_preserves_exact_utf8_bytes_and_unchanged_payload() -> TestResult {
    let body = format!("{PREFIX}{}</ul>", row(INNER));
    let RosterVerdict::Complete { roster } = parse_roster(&body, team()?)? else {
        return Err("readable located row must be complete".into());
    };
    check!(eq; roster.athletes.len(), 1);
    let member = roster.athletes.first().ok_or("missing positive member")?;
    check_member(
        &body,
        member,
        0,
        PREFIX.len().checked_add(OPEN.len()).ok_or("offset")?,
        INNER,
    )?;
    check!(eq; member.athlete(), &RosterAthlete {
        roster_name: "D'Arc, José".to_string(),
        name: "José D'Arc".to_string(),
        gender: Gender::Girls,
        grad_year: GradYear::CO2027,
        athlete_id: "42".to_string(),
        profile_url: "https://wi.milesplit.com/athletes/42-example".to_string(),
        indoor: true,
        outdoor: false,
        xc: false,
    });
    Ok(())
}

#[test]
fn partial_roster_keeps_global_ordinals_and_duplicate_occurrence_positions() -> TestResult {
    let first = row(INNER);
    let rejected_row = row(REJECTED);
    let middle = "é
";
    let body = format!("{PREFIX}{first}{rejected_row}{middle}{first}</ul>");
    let RosterVerdict::Partial {
        roster,
        rejected,
        unfinished,
    } = parse_roster(&body, team()?)?
    else {
        return Err("one invalid row must retain both positive occurrences".into());
    };
    let [first, second] = roster.athletes.as_slice() else {
        return Err("expected two positive occurrences".into());
    };
    let [rejection] = rejected.as_slice() else {
        return Err("expected one rejected occurrence".into());
    };
    check!(eq; unfinished, None);
    check!(eq; first.athlete(), second.athlete());
    let first_offset = PREFIX.len().checked_add(OPEN.len()).ok_or("first offset")?;
    let rejection_offset = first_offset
        .checked_add(row(INNER).len())
        .ok_or("rejected offset")?;
    let second_offset = rejection_offset
        .checked_add(rejected_row.len())
        .and_then(|offset| offset.checked_add(middle.len()))
        .ok_or("second offset")?;
    check_member(&body, first, 0, first_offset, INNER)?;
    check_member(&body, second, 2, second_offset, INNER)?;
    check!(eq; rejection.row, SourceRowLocator {
        ordinal: 1, byte_offset: rejection_offset, byte_length: REJECTED.len(),
    });
    check!(eq; rejection.kind, RosterRejectionKind::InvalidGraduationYear);
    check!(eq; rejection.athlete_id.as_deref(), Some("99"));
    check!(eq; span(&body, rejection.row)?, REJECTED);
    Ok(())
}

#[test]
fn positive_roster_locator_capacity_keeps_only_the_existing_bounded_prefix() -> TestResult {
    let row = row(INNER);
    for count in [20_000usize, 20_001] {
        let body = format!("{PREFIX}{}</ul>", row.repeat(count));
        let verdict = parse_roster(&body, team()?)?;
        check!(eq; matches!(&verdict, RosterVerdict::Complete { .. }), count == 20_000);
        let roster = verdict.roster().ok_or("bounded prefix was lost")?;
        check!(eq; roster.athletes.len(), 20_000);
        check!(verdict.rejections().is_empty());
        let last = roster
            .athletes
            .last()
            .ok_or("missing bounded last member")?;
        let offset = row
            .len()
            .checked_mul(19_999)
            .and_then(|offset| offset.checked_add(PREFIX.len()))
            .and_then(|offset| offset.checked_add(OPEN.len()))
            .ok_or("last offset")?;
        check_member(&body, last, 19_999, offset, INNER)?;
        match verdict.unfinished() {
            Some(locator) => {
                check!(eq; count, 20_001);
                check!(eq; locator, SourceRowLocator {
                    ordinal: 20_000,
                    byte_offset: offset.checked_add(row.len()).ok_or("unfinished offset")?,
                    byte_length: INNER.len(),
                });
                check!(eq; span(&body, locator)?, INNER);
            }
            None => check!(eq; count, 20_000),
        }
    }
    Ok(())
}

#[test]
fn canonical_roster_projection_is_unchanged_by_positive_locator_offsets() -> TestResult {
    let team = team()?;
    let original = parse_roster(ROSTER, team.clone())?;
    let prefix = "é🏃
";
    let moved = parse_roster(&format!("{prefix}{ROSTER}"), team)?;
    let original = original.roster().ok_or("missing original roster")?;
    let moved = moved.roster().ok_or("missing moved roster")?;
    let site = Site::for_jurisdiction(UsJurisdiction::Wisconsin);
    let project = |roster: &Roster| {
        roster_entities(
            &roster.team,
            &roster.athletes,
            SchoolYear::DEFAULT,
            "2026-10-07",
            &site,
        )
    };
    check!(eq; serde_json::to_vec(&project(original)?)?, serde_json::to_vec(&project(moved)?)?);
    check!(eq; original.athletes.len(), moved.athletes.len());
    for (original, moved) in original.athletes.iter().zip(&moved.athletes) {
        check!(eq; original.athlete(), moved.athlete());
        let original = original.locator();
        check!(eq; moved.locator(), SourceRowLocator {
            ordinal: original.ordinal,
            byte_offset: original.byte_offset.checked_add(prefix.len()).ok_or("moved offset")?,
            byte_length: original.byte_length,
        });
    }
    Ok(())
}
