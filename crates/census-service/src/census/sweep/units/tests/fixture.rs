use super::*;
use census_crawl::net::FetchOutcome;
use sha2::{Digest, Sha256};
use std::num::NonZeroU32;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn team(id: &str) -> TeamRef {
    TeamRef {
        id: id.into(),
        slug: "controlled".into(),
        url: format!("https://mi.milesplit.com/teams/{id}-controlled"),
        name: "Controlled School".into(),
        city_state: "Controlled, MI".into(),
    }
}

pub(super) fn revision() -> TestResult<NonZeroU32> {
    NonZeroU32::new(1).ok_or_else(|| "controlled revision".into())
}

pub(super) fn inputs_at(bytes: usize) -> TestResult<Vec<TeamRef>> {
    let mut teams: Vec<_> = (1..=1024)
        .map(|id| {
            let mut row = team(&id.to_string());
            row.name.clear();
            row
        })
        .collect();
    let fixed = teams.iter().try_fold(0usize, |total, row| {
        [&row.id, &row.slug, &row.url, &row.city_state]
            .into_iter()
            .try_fold(
                total
                    .checked_add(std::mem::size_of::<TeamRef>())
                    .ok_or("fixed size")?,
                |size, field| size.checked_add(field.len()).ok_or("field size"),
            )
    })?;
    let remaining = bytes
        .checked_sub(fixed)
        .ok_or("target smaller than fixed inputs")?;
    let each = remaining
        .checked_div(teams.len())
        .ok_or("empty input count")?;
    let rest = remaining
        .checked_rem(teams.len())
        .ok_or("empty input count")?;
    for (index, row) in teams.iter_mut().enumerate() {
        let length = each
            .checked_add(usize::from(index < rest))
            .ok_or("padding overflow")?;
        check!(length <= 4096);
        row.name = "x".repeat(length);
    }
    Ok(teams)
}

pub(super) fn raw_done(store: &Store, team: &TeamRef) -> TestResult {
    let body = b"controlled old roster terminal protocol fixture";
    let capture = FetchOutcome {
        url: format!("{}/roster", team.url),
        response_url: Some(format!("{}/roster", team.url)),
        method: "GET".into(),
        status: 200,
        content_digest: format!("{:x}", Sha256::digest(body)),
        bytes: body.len(),
        fetched_at: "2026-10-07T12:00:00Z".into(),
        from_cache: true,
        content_type: Some("text/html".into()),
        body: body.to_vec(),
    };
    let phase =
        crate::census::rosters_phase(UsJurisdiction::Michigan, SchoolYear::DEFAULT, revision()?);
    store.journal_done(
        &phase,
        &format!("MI:{}", team.id),
        &serde_json::json!({
            "team_id":team.id, "school":null, "year":SchoolYear::DEFAULT.get(),
            "observed_on":"2026-10-07", "capture":capture, "refusal":null,
            "quarantine":null, "rejected":[], "unfinished":null, "disposition":"complete",
            "athletes":0, "teams":0, "co2027":0, "co2027_boys":0, "co2027_girls":0
        }),
    )?;
    Ok(())
}
