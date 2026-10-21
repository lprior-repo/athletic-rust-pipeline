use super::*;
use census_domain::model::SourceAthleteObservation;

pub(in super::super) fn verify(snapshot: &StoreSnapshot<'_>, reference: &str) -> TestResult {
    let mut rows: Vec<SourceAthleteObservation> = Vec::new();
    let mut schools = 0usize;
    snapshot.for_each_observation(Table::SourceObservations, |row: SourceObservation| {
        match row {
            SourceObservation::Athlete(athlete) => rows.push(athlete),
            SourceObservation::School(_) => schools = schools.saturating_add(1),
        }
        Ok(())
    })?;
    check!(eq; schools, 1);
    check!(eq; rows.len(), 3);
    rows.sort_by(|left, right| left.source_row_key.cmp(&right.source_row_key));
    let mut expected = [
        (format!("provider_target_intake_v1:{reference}:profile:html_bytes:32849:32912"),
            Gender::Unknown, "2026-10-16T00:50:27+00:00"),
        (format!("provider_target_intake_v1:{reference}:milesplit_meet_team_result_competition_category:html_bytes:65480:66799"),
            Gender::Boys, "2026-10-16T00:49:19+00:00"),
        (format!("provider_target_intake_v1:{reference}:milesplit_meet_team_result_competition_category:html_bytes:69033:70352"),
            Gender::Boys, "2026-10-16T00:49:19+00:00"),
    ];
    expected.sort_by(|left, right| left.0.cmp(&right.0));
    for (row, (key, category, at)) in rows.iter().zip(expected) {
        check!(eq; row.namespace, SourceNamespace::MilesplitAthlete);
        check!(eq; row.source_athlete_id, "7223777");
        check!(eq; row.observed_name, "Zacchaeus Brocks");
        check!(eq; row.observed_school.as_deref(), Some("Detroit Catholic Central"));
        check!(eq; row.profile_url.as_deref(), Some("https://mi.milesplit.com/athletes/7223777-zacchaeus-brocks"));
        check!(eq; row.source_row_key, key);
        check!(eq; row.gender, category);
        check!(eq; row.observed_on, at);
        check!(eq; row.observed_grade, None);
    }
    Ok(())
}
