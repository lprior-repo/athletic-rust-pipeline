//! The athlete half of the observation funnel.
//!
//! [`observe_schools_of`](crate::observe_schools_of) records what a source published about its
//! schools; this module records the same for the athletes a pass read, in the module's own file only
//! because `lib.rs` sits on the repository's source-length budget. It is not a second kind of write:
//! both halves append `Table::SourceObservations` rows keyed by the provider's own object id, which
//! is what lets a canonical merge be re-decided from what each source said rather than by reading
//! the provider again.

use crate::CrawlResult;
use census_domain::model::{
    CanonicalAthlete, CanonicalPerformance, CanonicalSchool, SourceAthleteObservation,
    SourceIdentity, SourceNamespace, SourceObservation,
};
use census_store::{Store, Table};
use std::collections::HashMap;

/// Stamp performances with the source's own athlete object: the §31 key (`namespace`, provider
/// athlete id) the athlete row already carries for this pass's source.
///
/// Called where a pass hands its rows to the store, which is the only place that holds both the
/// athletes it just minted and the performances that name them. A row that already carries an
/// identity keeps it, and a row whose athlete has no identity in `namespace` stays `None` rather
/// than borrowing the canonical id — the merge the identity exists to outlive is exactly what a
/// canonical id cannot outlive.
///
/// Returns how many rows were stamped.
pub fn stamp_source_athletes<'a>(
    namespace: &SourceNamespace,
    athletes: impl IntoIterator<Item = &'a CanonicalAthlete>,
    performances: &mut [CanonicalPerformance],
) -> usize {
    let identities: HashMap<&str, &SourceIdentity> = athletes
        .into_iter()
        .filter_map(|athlete| {
            athlete
                .identity_in(namespace)
                .map(|identity| (athlete.id.as_str(), identity))
        })
        .collect();
    let mut stamped = 0usize;
    for performance in performances {
        if performance.source_athlete.is_some() {
            continue;
        }
        if let Some(identity) = identities.get(performance.athlete.as_str()) {
            performance.source_athlete = Some((*identity).clone());
            stamped = stamped.saturating_add(1);
        }
    }
    stamped
}

/// The athlete half of the funnel, for the callers that hold a store rather than an
/// [`AdapterContext`](crate::AdapterContext).
///
/// `namespace` is the provider object the pass just read, never one inferred from the rows, and
/// `schools` are the school rows that pass placed its athletes at, in preference order: the first row
/// carrying an athlete's school id names the school that athlete was observed at. An athlete whose
/// school appears in neither writes an observation with no school rather than one naming a school the
/// source did not place them at.
pub fn observe_athletes_of<'a>(
    store: &Store,
    namespace: &SourceNamespace,
    athletes: &[CanonicalAthlete],
    schools: impl IntoIterator<Item = &'a CanonicalSchool>,
    observed_on: &str,
) -> CrawlResult<usize> {
    let mut names: HashMap<&str, &str> = HashMap::new();
    for school in schools {
        names
            .entry(school.id.as_str())
            .or_insert(school.name.as_str());
    }
    let rows: Vec<SourceObservation> = athletes
        .iter()
        .filter_map(|athlete| {
            let school = names
                .get(athlete.school.as_str())
                .map(|name| (*name).to_string());
            SourceAthleteObservation::of_athlete(namespace, athlete, school, observed_on)
        })
        .map(SourceObservation::Athlete)
        .collect();
    store.append_many(Table::SourceObservations, &rows)?;
    Ok(rows.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::{
        CanonicalEvent, CanonicalMeet, CanonicalTeam, EventKind, Gender, GradYear, Mark,
        SchoolYear, Sport, TimingMethod,
    };
    use census_domain::UsJurisdiction;

    /// One athlete carrying the provider id `source` under `namespace`, and no other identity.
    fn athlete(name: &str, source: &str, namespace: SourceNamespace) -> CanonicalAthlete {
        let school = CanonicalSchool::mint(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
        let mut row = CanonicalAthlete::new(&school, name, GradYear::CO2027, Gender::Boys);
        row.source_identities
            .push(SourceIdentity::new(namespace, source));
        row
    }

    /// One performance of that athlete, minted the way an adapter mints it.
    fn performance_of(athlete: &CanonicalAthlete, source_key: &str) -> CanonicalPerformance {
        let school = CanonicalSchool::mint(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
        let meet = CanonicalMeet::mint(
            Some(UsJurisdiction::Wisconsin),
            "2026-05-01",
            "Abbotsford Invite",
            None,
        );
        let team = CanonicalTeam::mint(
            &school,
            Sport::OutdoorTrack,
            Gender::Boys,
            SchoolYear::new(2026).expect("2026"),
        );
        CanonicalPerformance {
            id: CanonicalPerformance::mint(
                &athlete.id,
                &meet,
                &EventKind::Track100m,
                "2026-05-01",
                source_key,
            ),
            athlete: athlete.id.clone(),
            team,
            event: CanonicalEvent::new(&meet, EventKind::Track100m, Gender::Boys, None, None).id,
            meet,
            date: "2026-05-01".to_string(),
            mark: Mark::TimeSeconds(10.94),
            wind_mps: None,
            place: None,
            heat: None,
            round: None,
            timing: Some(TimingMethod::Fat),
            observed_grade: None,
            evidence: Vec::new(),
            source_key: source_key.to_string(),
            source_athlete: None,
            retained_conflicts: Vec::new(),
        }
    }

    #[test]
    fn stamping_names_the_source_athlete_only_where_the_row_holds_one() {
        let known = athlete(
            "Julian Aguilera",
            "998877",
            SourceNamespace::MilesplitAthlete,
        );
        let unknown = athlete("Mateo Barrera", "555", SourceNamespace::TfrrsAthlete);
        let mut rows = vec![
            performance_of(&known, "perf-1"),
            performance_of(&unknown, "perf-2"),
        ];

        let stamped = stamp_source_athletes(
            &SourceNamespace::MilesplitAthlete,
            [&known, &unknown],
            &mut rows,
        );

        assert_eq!(
            stamped, 1,
            "only the athlete known to that source is stamped"
        );
        assert_eq!(
            rows[0]
                .source_athlete
                .as_ref()
                .map(|identity| identity.id.as_str()),
            Some("998877")
        );
        assert_eq!(
            rows[1].source_athlete, None,
            "an athlete the source never named stays unnamed rather than borrowing the canonical id"
        );
    }

    #[test]
    fn stamping_never_overwrites_an_identity_a_row_already_carries() {
        let known = athlete(
            "Julian Aguilera",
            "998877",
            SourceNamespace::MilesplitAthlete,
        );
        let other = athlete("Mateo Barrera", "776655", SourceNamespace::TfrrsAthlete);
        let mut rows = vec![performance_of(&other, "perf-3")];
        rows[0].source_athlete = Some(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "776655"));

        let stamped = stamp_source_athletes(
            &SourceNamespace::MilesplitAthlete,
            [&known, &other],
            &mut rows,
        );

        assert_eq!(stamped, 0);
        assert_eq!(
            rows[0]
                .source_athlete
                .as_ref()
                .map(|identity| identity.namespace.clone()),
            Some(SourceNamespace::TfrrsAthlete),
            "the row keeps the identity it was written with"
        );
    }
}
