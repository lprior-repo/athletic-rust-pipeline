use super::*;
use crate::milesplit::map::{absorb_result_set, OwnedResultSet};
use crate::milesplit::owned::{parse_owned_meet, OwnedMeetOutcome, OwnedMeetVerdict};
use crate::milesplit::raw::RawPage;
use crate::milesplit::results::{Accumulator, Stats};
use crate::milesplit::wire::ResultSetRef;
use crate::net::cache::content_digest;
use crate::net::FetchOutcome;
use census_domain::model::{CentiMetres, EventKind, Mark, SchoolYear, SourceIdentity};
use census_domain::UsJurisdiction;
use serde_json::{json, Value};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const TROY: &[u8] = include_bytes!("../../owned/fixtures/troy_725218.json");

mod binding;
mod cohorts;
mod unresolved;

fn document() -> TestResult<Value> {
    Ok(serde_json::from_slice(TROY)?)
}

fn school(name: &str, team: &str) -> CanonicalSchool {
    let mut school = CanonicalSchool::new(
        UsJurisdiction::Alabama,
        name,
        census_domain::model::normalize_name(name),
        None,
    )
    .0;
    school
        .source_identities
        .push(SourceIdentity::new(SourceNamespace::MilesplitSchool, team));
    school
}

fn metadata() -> TestResult<RawPage> {
    Ok(crate::milesplit::raw::parse_raw(
        include_str!(
            "../../../../tests/fixtures/milesplit/troy_725218_rs1266814_raw_projection.html"
        ),
        "https://al.milesplit.com/meets/725218/results/1266814/raw",
    )?)
}

fn project(
    document: Value,
    schools: &[CanonicalSchool],
    raw: RawPage,
) -> TestResult<(Accumulator, Stats)> {
    let body = serde_json::to_vec(&document)?;
    let outcome = OwnedMeetOutcome {
        verdict: parse_owned_meet(&body, 725218),
        capture: FetchOutcome {
            url: "https://al.milesplit.com/api/v1/meets/725218/performances".into(),
            response_url: None,
            method: "GET".into(),
            status: 200,
            content_digest: content_digest(&body),
            bytes: body.len(),
            fetched_at: "2026-10-01T23:44:16Z".into(),
            from_cache: true,
            content_type: Some("application/json".into()),
            body,
        },
    };
    let OwnedMeetVerdict::Parsed(page) = &outcome.verdict else {
        return Err("parsed fixture".into());
    };
    let indices: Vec<_> = page
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.result_set_id == 1266814)
        .map(|(index, _)| index)
        .collect();
    let reference =
        ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
            .ok_or("reference")?;
    let mut accumulated = Accumulator::default();
    let mut stats = Stats::default();
    absorb_result_set(
        &raw,
        &reference,
        OwnedResultSet {
            capture: &outcome.capture,
            page,
            indices: &indices,
        },
        &ProviderSchools::from_schools(schools),
        &mut stats,
        &mut accumulated,
    )?;
    Ok((accumulated, stats))
}

#[test]
fn spann_long_and_triple_jump_survive_with_original_provider_owner_and_acquisition_evidence(
) -> TestResult {
    let bound = school("Provider school not joined by display name", "38332");
    let (accumulated, _) = project(
        document()?,
        &[bound.clone(), bound.clone(), school("Charles", "4912")],
        metadata()?,
    )?;
    let athlete = accumulated
        .athletes
        .values()
        .find(|athlete| {
            athlete
                .source
                .as_ref()
                .is_some_and(|source| source.id == "14222592")
        })
        .ok_or("SPANN provider owner")?;
    check!(eq; athlete.school, bound.id);
    check!(eq;
        athlete.grad_year,
        census_domain::model::GradYear::new(2027).ok_or("cohort")?
    );
    check!(eq; athlete.observed_grades, Vec::new());
    let authentic_season = SchoolYear::new(2025).ok_or("authentic season")?;
    check!(accumulated
        .teams
        .values()
        .all(|team| team.school_year == authentic_season));
    check!(eq;
        athlete.public_profile_urls,
        vec!["https://www.milesplit.com/athletes/14222592-adelyn-spann"]
    );
    for (source_key, kind, raw, centimetres, place, locator) in [
        (
            "milesplit_result:201782263",
            EventKind::LongJump,
            "13-9",
            419,
            11,
            "data[0]",
        ),
        (
            "milesplit_result:201782277",
            EventKind::TripleJump,
            "30-7",
            932,
            5,
            "data[1]",
        ),
    ] {
        let performance = accumulated
            .performances
            .values()
            .find(|performance| performance.source_key == source_key)
            .ok_or("observed mark")?;
        check!(eq; performance.athlete, athlete.id);
        check!(eq;
            performance.mark,
            Mark::FieldImperial {
                feet_mark: raw.into(),
                metres: CentiMetres::new(centimetres),
            }
        );
        check!(eq; accumulated.events[performance.event.as_str()].kind, kind);
        check!(eq; performance.place, Some(place));
        check!(eq; performance.heat.as_deref(), Some("1"));
        check!(eq; performance.round.as_deref(), Some("Finals"));
        check!(eq; performance.date, "2026-03-27");
        check!(eq; performance.observed_grade, None);
        let evidence = &performance.evidence[0];
        let note: Value = serde_json::from_str(evidence.note.as_deref().ok_or("provenance")?)?;
        check!(eq; note["locator"], locator);
        check!(eq; note["provider"]["mark"], raw);
        check!(eq; note["team_id"], 38332);
        check!(eq;
            note["sha256"],
            content_digest(&serde_json::to_vec(&document()?)?)
        );
        check!(eq; note["acquired_at"], "2026-10-01T23:44:16Z");
        check!(eq; evidence.observed_on, "2026-10-01T23:44:16Z");
        check!(eq;
            evidence.source.url.as_deref(),
            Some("https://al.milesplit.com/api/v1/meets/725218/performances")
        );
    }
    Ok(())
}

#[test]
fn absent_sport_metadata_preserves_source_athlete_without_inventing_a_team_or_comparable_mark(
) -> TestResult {
    let mut raw = metadata()?;
    raw.sport = None;
    let (accumulated, _) = project(document()?, &[school("Spann school", "38332")], raw)?;
    let athlete = accumulated
        .athletes
        .values()
        .find(|athlete| athlete.canonical_name == "Adelyn Spann")
        .ok_or("source-owned cohort/name survives")?;
    check!(eq; athlete.sports, Vec::new());
    check!(eq; accumulated.performances, std::collections::HashMap::new());
    check!(eq; accumulated.teams, std::collections::HashMap::new());
    let retained = accumulated
        .retained
        .values()
        .find(|row| row["result_id"] == 201782263)
        .ok_or("missing sport observation")?;
    check!(eq;
        retained["reasons"],
        json!(["raw metadata does not publish a supported sport"])
    );
    Ok(())
}
