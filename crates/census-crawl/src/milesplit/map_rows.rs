use super::{owned, record_event, team_for, MeetContext, OwnedPerformance, RowWriter};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{AthleteId, CanonicalAthlete, CanonicalPerformance, Evidence, SchoolId};

pub(super) fn record_row(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &OwnedPerformance,
) -> CrawlResult<usize> {
    bump(&mut writer.stats.rows);
    let name = format!("{} {}", row.first_name.trim(), row.last_name.trim());
    let key = owned::source_key(row);
    writer
        .accumulated
        .observations
        .entry(key)
        .or_insert_with(|| owned::observation(context, row, name.trim()));
    let evidence = owned::evidence(context, row);
    let schools = writer.schools;
    let school = schools.resolve(row.team_id);
    let reasons = retention_reasons(writer, context, row, school, name.trim());
    writer.accumulated.retained.insert(
        format!(
            "observation/{}/{}/{}",
            row.meet_id, row.result_id, context.capture.content_digest
        ),
        serde_json::json!({
            "disposition": if reasons.is_empty() { "projected" } else { "retained_unresolved" },
            "source_athlete": row.source_athlete, "result_id": row.result_id,
            "team_id": row.team_id, "locator": row.locator, "cohort": row.cohort,
            "reasons": reasons, "evidence": evidence,
        }),
    );
    let (Some(school), Some(year)) = (school, row.grad_year) else {
        return Ok(0);
    };
    if name.trim().is_empty() {
        return Ok(0);
    }
    let athlete = record_athlete(writer, context, row, school, name.trim(), year, &evidence)?;
    let Some(sport) = context.sport else {
        return Ok(1);
    };
    let team = team_for(
        &mut writer.accumulated.teams,
        school,
        sport,
        row.gender,
        context.school_year,
        row,
        &evidence,
    );
    let event = record_event(writer, context, row, &evidence);
    record_performance(writer, context, row, &athlete, &team, &event, evidence)?;
    Ok(1)
}

fn retention_reasons(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &OwnedPerformance,
    school: Option<&SchoolId>,
    name: &str,
) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if row.grad_year.is_none() {
        bump(&mut writer.stats.rows_without_cohort);
        reasons.push(match row.cohort {
            super::super::owned::OwnedCohort::Invalid => {
                "invalid published cohort; no grade inference"
            }
            _ => "missing published cohort; no grade inference",
        });
    }
    if row.grad_year.is_some() {
        bump(&mut writer.stats.rows_with_cohort);
    }
    if school.is_none() {
        bump(&mut writer.stats.rows_without_school);
        let reason = writer.schools.unresolved_reason(row.team_id);
        let counter = writer
            .stats
            .unresolved
            .entry(format!("teamID {}: {reason}", row.team_id))
            .or_default();
        bump(counter);
        reasons.push(reason);
    } else {
        bump(
            writer
                .stats
                .school_resolved
                .entry("exact_unique_provider_team_id")
                .or_default(),
        );
    }
    if context.sport.is_none() {
        bump(&mut writer.stats.rows_without_sport);
        reasons.push("raw metadata does not publish a supported sport");
    }
    if name.is_empty() {
        bump(&mut writer.stats.rows_without_name);
        reasons.push("empty published athlete name");
    }
    reasons
}

#[allow(clippy::too_many_arguments)]
fn record_athlete(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &OwnedPerformance,
    school: &SchoolId,
    name: &str,
    year: census_domain::model::GradYear,
    evidence: &Evidence,
) -> CrawlResult<AthleteId> {
    let id = CanonicalAthlete::mint(school, name, year, row.gender, &row.source_athlete);
    let athlete = match writer.accumulated.athletes.entry(id.as_str().to_string()) {
        std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
        std::collections::hash_map::Entry::Vacant(entry) => {
            let mut athlete = CanonicalAthlete::new_checked(
                school,
                name,
                year,
                row.gender,
                row.source_athlete.clone(),
            )
            .map_err(|detail| CrawlError::Invariant { detail })?;
            athlete
                .public_profile_urls
                .extend(row.source_athlete.url.clone());
            entry.insert(athlete)
        }
    };
    if let Some(sport) = context
        .sport
        .filter(|sport| !athlete.sports.contains(sport))
    {
        athlete.sports.push(sport);
    }
    owned::record_published_graduation(athlete, row, &evidence.source);
    athlete.evidence.push(evidence.clone());
    Ok(id)
}

#[allow(clippy::too_many_arguments)]
fn record_performance(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &OwnedPerformance,
    athlete: &AthleteId,
    team: &census_domain::model::TeamId,
    event: &census_domain::model::EventId,
    evidence: Evidence,
) -> CrawlResult<()> {
    let mut performance = CanonicalPerformance::new_checked(
        athlete,
        &row.event_kind,
        team,
        event,
        &context.meet.id,
        &context.meet.date,
        row.mark.clone(),
        &owned::source_key(row),
        owned::number(row, "windReading"),
        owned::place(row),
    )
    .map_err(|detail| CrawlError::Invariant { detail })?;
    performance.heat = owned::scalar(row, "heat");
    performance.round = owned::text(row, "roundName").map(str::to_string);
    performance.timing = row.timing;
    performance.source_athlete = Some(row.source_athlete.clone());
    performance.evidence = vec![evidence, context.metadata.clone()];
    writer
        .accumulated
        .performances
        .entry(performance.id.as_str().into())
        .or_insert(performance);
    Ok(())
}

fn bump(counter: &mut usize) {
    *counter = counter.saturating_add(1);
}
