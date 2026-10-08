use super::{OwnedPerformance, SourceContext};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Evidence, PublishedGraduation, SchoolId,
    SourceAthleteObservation, SourceNamespace, SourceObservation, SourceRef,
};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub(in crate::milesplit) struct ProviderSchools {
    bindings: HashMap<u64, Option<SchoolId>>,
}

impl ProviderSchools {
    pub(in crate::milesplit) fn from_schools(schools: &[CanonicalSchool]) -> Self {
        let mut bindings = HashMap::new();
        schools.iter().for_each(|school| {
            school
                .source_identities
                .iter()
                .filter(|identity| identity.namespace == SourceNamespace::MilesplitSchool)
                .filter_map(|identity| positive_id(&identity.id))
                .for_each(|id| {
                    let entry = bindings
                        .entry(id)
                        .or_insert_with(|| Some(school.id.clone()));
                    if entry.as_ref().is_some_and(|owner| owner != &school.id) {
                        *entry = None;
                    }
                });
        });
        Self { bindings }
    }

    pub(super) fn resolve(&self, team_id: u64) -> Option<&SchoolId> {
        self.bindings.get(&team_id).and_then(Option::as_ref)
    }

    pub(super) fn unresolved_reason(&self, team_id: u64) -> &'static str {
        match self.bindings.get(&team_id) {
            Some(None) => "ambiguous MilesplitSchool provider teamID",
            _ => "missing exact positive MilesplitSchool provider teamID",
        }
    }
}

fn positive_id(token: &str) -> Option<u64> {
    token.parse::<u64>().ok().filter(|id| {
        *id > 0 && !token.starts_with('0') && token.bytes().all(|byte| byte.is_ascii_digit())
    })
}

pub(super) fn text<'a>(row: &'a OwnedPerformance, field: &str) -> Option<&'a str> {
    row.provider
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

pub(super) fn scalar(row: &OwnedPerformance, field: &str) -> Option<String> {
    match row.provider.get(field)? {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

pub(super) fn number(row: &OwnedPerformance, field: &str) -> Option<f64> {
    let value = row.provider.get(field)?;
    value
        .as_f64()
        .or_else(|| value.as_str()?.parse().ok())
        .filter(|value| value.is_finite())
}

pub(super) fn place(row: &OwnedPerformance) -> Option<u16> {
    let value = row.provider.get("place")?;
    let place = value.as_u64().or_else(|| value.as_str()?.parse().ok())?;
    u16::try_from(place).ok()
}

pub(super) fn source_key(row: &OwnedPerformance) -> String {
    format!("milesplit_result:{}", row.result_id)
}

pub(super) fn evidence(context: &SourceContext<'_>, row: &OwnedPerformance) -> Evidence {
    let mut evidence = Evidence::parsed(
        SourceRef::new(
            context.reference.site.source_id(),
            Some(context.capture.url.clone()),
        ),
        &context.capture.fetched_at,
    );
    evidence.note = Some(producer_note(context, row));
    evidence
}

fn producer_note(context: &SourceContext<'_>, row: &OwnedPerformance) -> String {
    serde_json::json!({
        "locator": row.locator, "result_id": row.result_id, "meet_id": row.meet_id,
        "result_set_id": row.result_set_id, "team_id": row.team_id,
        "source_athlete": row.source_athlete, "cohort": row.cohort,
        "published_grad_year": row.grad_year, "provider": row.provider,
        "capture_url": context.capture.url, "sha256": context.capture.content_digest,
        "acquired_at": context.capture.fetched_at,
        "raw_metadata_url": context.reference.url,
        "meet_name": context.page.meet.name, "meet_date": context.page.meet.date,
        "sport": context.page.sport, "school_year": context.page.school_year,
        "identity_decision": "none; source-owned observation only",
        "best_mark_scope": "observed performance; not lifetime PR",
    })
    .to_string()
}

pub(super) fn record_published_graduation(
    athlete: &mut CanonicalAthlete,
    row: &OwnedPerformance,
    source: &SourceRef,
) {
    if row.cohort != crate::milesplit::owned::OwnedCohort::Published {
        return;
    }
    let Some(grad_year) = row.grad_year else {
        return;
    };
    if athlete
        .published_graduations
        .iter()
        .any(|claim| claim.grad_year == grad_year && &claim.source == source)
    {
        return;
    }
    athlete.published_graduations.push(PublishedGraduation {
        grad_year,
        source: source.clone(),
    });
}

pub(super) fn observation(
    context: &SourceContext<'_>,
    row: &OwnedPerformance,
    name: &str,
) -> SourceObservation {
    SourceObservation::Athlete(
        SourceAthleteObservation::new(
            row.source_athlete.namespace.clone(),
            &row.source_athlete.id,
            source_key(row),
            name,
            &context.capture.fetched_at,
        )
        .with_gender(row.gender)
        .with_profile_url(row.source_athlete.url.clone())
        .with_school(text(row, "teamName").map(str::to_string)),
    )
}

#[cfg(test)]
mod tests;
