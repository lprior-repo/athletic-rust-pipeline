use super::super::provider_school::ProviderSchoolPacketV1;
use super::super::RetainedHtmlCaptureV1;
use crate::milesplit::{roster_entities, RosterVerdict, Site};
use crate::net::FetchOutcome;
use crate::recording::RowSink;
use crate::{CrawlError, CrawlResult, Recording};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, SchoolYear, SourceAthleteObservation, SourceNamespace,
    SourceObservation,
};
use census_store::Table;

const MAX_BODY: usize = 8 * 1024 * 1024;
const MAX_FIELD: usize = 4096;

pub fn record_provider_roster_intake(
    store: &census_store::Store,
    recording: &Recording,
    packet: &ProviderSchoolPacketV1,
) -> CrawlResult<()> {
    let checked = packet.read()?;
    let roster_verdict = checked.roster_verdict();
    match roster_verdict {
        RosterVerdict::Complete { roster } => {
            let roster_capture = packet.roster();
            let body = roster_capture.body()?;
            let fetch = fetch_outcome(roster_capture, &body)?;
            let site = Site::for_jurisdiction(
                checked
                    .school()
                    .state
                    .ok_or_else(|| invalid("provider roster jurisdiction missing"))?,
            );
            let observed_on = roster_capture
                .acquired_at()
                .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
            let date = roster_capture.acquired_at().format("%Y-%m-%d").to_string();
            let year = SchoolYear::from_date(&date).ok_or_else(|| CrawlError::Invariant {
                detail: "provider roster acquisition year unsupported".into(),
            })?;
            let team = &roster.team;
            let members = &roster.athletes;
            let (school, mut athletes, teams) =
                roster_entities(team, members, year, &observed_on, &site)?;
            let mut observations = athlete_observations(&athletes, &school, &observed_on)?;
            if members.len() != athletes.len() || members.len() != observations.len() {
                return Err(CrawlError::Invariant {
                    detail: "roster projection cardinality changed".into(),
                });
            }
            let binding = super::roster_provenance::Binding::new(&fetch, team, roster_verdict)?;
            let result = members
                .iter()
                .zip(athletes.iter_mut())
                .zip(observations.iter_mut())
                .try_for_each(|((member, athlete), observation)| {
                    let SourceObservation::Athlete(observation) = observation else {
                        return Err(CrawlError::Invariant {
                            detail: "roster athlete observation has wrong kind".into(),
                        });
                    };
                    binding.attach(member, athlete, observation)
                });
            result?;
            let mut batch = RowSink::Record { store, recording }.write_batch();
            batch.append_many(Table::Teams, &teams)?;
            batch.append_many(Table::Athletes, &athletes)?;
            for observation in observations {
                batch.append_many(
                    Table::SourceObservations,
                    std::slice::from_ref(&observation),
                )?;
            }
            batch.commit()?;
        }
        RosterVerdict::Partial { .. } => {
            return Err(CrawlError::Invariant {
                detail: "provider roster is partial".into(),
            });
        }
        RosterVerdict::Quarantined { reason, .. } => {
            return Err(CrawlError::Invariant {
                detail: format!("provider roster is quarantined: {reason:?}"),
            });
        }
    }
    Ok(())
}

fn fetch_outcome(capture: &RetainedHtmlCaptureV1, body: &str) -> CrawlResult<FetchOutcome> {
    if body.len() > MAX_BODY {
        return Err(CrawlError::Resource {
            resource: "roster body bytes",
            requested: body.len(),
            limit: MAX_BODY,
        });
    }
    if capture.requested_url().len() > MAX_FIELD {
        return Err(CrawlError::Resource {
            resource: "roster URL bytes",
            requested: capture.requested_url().len(),
            limit: MAX_FIELD,
        });
    }
    let mut original = Vec::new();
    original
        .try_reserve_exact(body.len())
        .map_err(|_| CrawlError::Invariant {
            detail: "roster original allocation refused".into(),
        })?;
    original.extend_from_slice(body.as_bytes());
    Ok(FetchOutcome {
        url: capture.requested_url().into(),
        response_url: Some(capture.requested_url().into()),
        method: capture.provenance().request_method.clone(),
        status: 200,
        content_digest: capture.body_sha256().into(),
        bytes: body.len(),
        fetched_at: capture
            .acquired_at()
            .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
        from_cache: false,
        content_type: Some(capture.provenance().content_type.clone()),
        body: original,
    })
}

fn invalid(detail: &str) -> CrawlError {
    CrawlError::Invariant {
        detail: detail.into(),
    }
}

fn athlete_observations(
    athletes: &[CanonicalAthlete],
    school: &CanonicalSchool,
    observed_on: &str,
) -> CrawlResult<Vec<SourceObservation>> {
    let school_name = school.name.clone();
    let mut rows = Vec::new();
    rows.try_reserve(athletes.len())
        .map_err(|_| CrawlError::Invariant {
            detail: "roster observation allocation refused".into(),
        })?;
    let result = athletes.iter().try_fold(rows, |mut rows, athlete| {
        let observation = SourceAthleteObservation::of_athlete(
            &SourceNamespace::MilesplitAthlete,
            athlete,
            Some(school_name.clone()),
            observed_on,
        )
        .ok_or_else(|| CrawlError::Invariant {
            detail: "roster athlete lacks provider identity".into(),
        })?;
        rows.push(SourceObservation::Athlete(observation));
        Ok(rows)
    });
    result
}
