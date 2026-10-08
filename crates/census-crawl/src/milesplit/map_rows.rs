use super::{
    owned, record_event, team_for, MeetContext, OwnedPerformance, RowWriter, SourceContext,
};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalPerformance, EventId, Evidence, GradYear,
    PerformanceIdentity, PerformanceResult, SchoolId, TeamId,
};

pub(super) fn record_row(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &OwnedPerformance,
) -> CrawlResult<usize> {
    let (name, evidence) = record_source(writer, context.source, row);
    if !super::dates::admit(writer, context.source, row)? {
        return Ok(0);
    }
    let schools = writer.schools;
    let (Some(school), Some(year)) = (schools.resolve(row.team_id), row.grad_year) else {
        return Ok(0);
    };
    if name.trim().is_empty() {
        return Ok(0);
    }
    let athlete = record_athlete(writer, context, row, (school, name.trim(), year), &evidence)?;
    let Some(sport) = context.source.page.sport else {
        return Ok(1);
    };
    let team = team_for(writer, context, row, (school, sport), &evidence);
    let event = record_event(writer, context, row, &evidence).map_err(|error| {
        retain_event_failure(writer, context, row, &error);
        error
    })?;
    record_performance(writer, context, row, (&athlete, &team, &event), evidence)?;
    Ok(1)
}

pub(super) fn record_source(
    writer: &mut RowWriter<'_>,
    context: &SourceContext<'_>,
    row: &OwnedPerformance,
) -> (String, Evidence) {
    bump(&mut writer.stats.rows);
    let name = format!("{} {}", row.first_name.trim(), row.last_name.trim());
    writer
        .accumulated
        .observations
        .entry(owned::source_key(row))
        .or_insert_with(|| owned::observation(context, row, name.trim()));
    let evidence = owned::evidence(context, row);
    let schools = writer.schools;
    let reasons = retention_reasons(
        writer,
        context,
        row,
        schools.resolve(row.team_id),
        name.trim(),
    );
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
    (name, evidence)
}

fn retention_reasons(
    writer: &mut RowWriter<'_>,
    context: &SourceContext<'_>,
    row: &OwnedPerformance,
    school: Option<&SchoolId>,
    name: &str,
) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    retain_cohort(writer, row, &mut reasons);
    retain_school(writer, row, school, &mut reasons);
    if context.page.sport.is_none() {
        bump(&mut writer.stats.rows_without_sport);
        reasons.push("raw metadata does not publish a supported sport");
    }
    if name.is_empty() {
        bump(&mut writer.stats.rows_without_name);
        reasons.push("empty published athlete name");
    }
    reasons
}

fn retain_school(
    writer: &mut RowWriter<'_>,
    row: &OwnedPerformance,
    school: Option<&SchoolId>,
    reasons: &mut Vec<&'static str>,
) {
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
}

fn retain_cohort(
    writer: &mut RowWriter<'_>,
    row: &OwnedPerformance,
    reasons: &mut Vec<&'static str>,
) {
    if row.grad_year.is_none() {
        bump(&mut writer.stats.rows_without_cohort);
        reasons.push(match row.cohort {
            super::super::owned::OwnedCohort::Invalid => {
                "invalid published cohort; no grade inference"
            }
            _ => "missing published cohort; no grade inference",
        });
    } else {
        bump(&mut writer.stats.rows_with_cohort);
    }
}

fn record_athlete(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &OwnedPerformance,
    subject: (&SchoolId, &str, GradYear),
    evidence: &Evidence,
) -> CrawlResult<AthleteId> {
    let (school, name, year) = subject;
    let id = CanonicalAthlete::mint(school, name, year, row.gender, &row.source_athlete);
    let athlete = match writer.accumulated.athletes.entry(id.as_str().to_string()) {
        std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(new_athlete(subject, row)?)
        }
    };
    if let Some(sport) = context
        .source
        .page
        .sport
        .filter(|sport| !athlete.sports.contains(sport))
    {
        athlete.sports.push(sport);
    }
    owned::record_published_graduation(athlete, row, &evidence.source);
    athlete.evidence.push(evidence.clone());
    Ok(id)
}

fn new_athlete(
    subject: (&SchoolId, &str, GradYear),
    row: &OwnedPerformance,
) -> CrawlResult<CanonicalAthlete> {
    let mut athlete = CanonicalAthlete::new_checked(
        subject.0,
        subject.1,
        subject.2,
        row.gender,
        row.source_athlete.clone(),
    )
    .map_err(|detail| CrawlError::Invariant { detail })?;
    athlete
        .public_profile_urls
        .extend(row.source_athlete.url.clone());
    Ok(athlete)
}

fn record_performance(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &OwnedPerformance,
    subject: (&AthleteId, &TeamId, &EventId),
    evidence: Evidence,
) -> CrawlResult<()> {
    let mut performance = mint_performance(context, row, subject)?;
    performance.heat = owned::scalar(row, "heat");
    performance.round = owned::text(row, "roundName").map(str::to_string);
    performance.timing = row.timing;
    performance.source_athlete = Some(row.source_athlete.clone());
    performance
        .evidence
        .try_reserve(2)
        .map_err(|_| CrawlError::Resource {
            resource: "milesplit performance evidence",
            requested: 2,
            limit: 2,
        })?;
    performance
        .evidence
        .extend([evidence, context.metadata.clone()]);
    writer
        .accumulated
        .performances
        .entry(performance.id.as_str().into())
        .or_insert(performance);
    Ok(())
}

fn mint_performance(
    context: &MeetContext<'_>,
    row: &OwnedPerformance,
    subject: (&AthleteId, &TeamId, &EventId),
) -> CrawlResult<CanonicalPerformance> {
    Ok(CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: subject.0,
            event: subject.2,
            meet: &context.meet.id,
            date: &context.meet.date,
            source_key: &owned::source_key(row),
        },
        PerformanceResult {
            team: subject.1,
            mark: row.mark.clone(),
            wind_mps: owned::number(row, "windReading"),
            place: owned::place(row),
        },
    )?)
}

fn retain_event_failure(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &OwnedPerformance,
    error: &CrawlError,
) {
    writer.accumulated.retained.insert(
        format!(
            "observation/{}/{}/{}",
            row.meet_id, row.result_id, context.source.capture.content_digest
        ),
        serde_json::json!({
            "disposition": "retained_unresolved", "reason": error.to_string(),
            "source_athlete": row.source_athlete, "result_id": row.result_id,
            "team_id": row.team_id, "locator": row.locator, "provider": row.provider,
            "evidence": owned::evidence(context.source, row),
        }),
    );
}

fn bump(counter: &mut usize) {
    *counter = counter.saturating_add(1);
}
