use super::super::canonical::Value;
use super::super::{labels, reach};
use super::{cells, Verifier};
use crate::bests::SharedSelection;
use census_domain::model::{CanonicalAthlete, IdentityStatus, Sport};

impl Verifier<'_, '_> {
    pub(super) fn values(&mut self, athlete: &CanonicalAthlete) -> Vec<Value> {
        let prs: Vec<&SharedSelection> = self.expectations.prs_of(athlete.id.as_str()).collect();
        let mut values = Vec::with_capacity(labels::ATHLETE_HEADERS.len());
        values.extend(self.identity(athlete));
        append_performance_cells(&mut values, athlete, &prs);
        values.extend(self.tally_cells(athlete));
        let reach = reach::Reach::of(self.expectations, athlete);
        append_reach(
            &mut values,
            reach,
            self.expectations.athletics_url(athlete.school.as_str()),
        );
        append_profiles(&mut values, athlete);
        values.extend(self.status_cells(athlete));
        self.append_address(&mut values, athlete);
        values
    }

    fn identity(&self, athlete: &CanonicalAthlete) -> Vec<Value> {
        let school = athlete.school.as_str();
        vec![
            Value::text(athlete.id.as_str()),
            Value::text(&athlete.canonical_name),
            Value::text(athlete.gender.stable_key()),
            Value::integer(i64::from(athlete.grad_year.get())),
            Value::optional(cells::observed_season(athlete).map(|year| year.short())),
            Value::text(self.expectations.school_state(school)),
            Value::text(self.expectations.school_name(school)),
            Value::text(self.expectations.school_id(school)),
            Value::text(self.expectations.school_city(school)),
        ]
    }

    fn tally_cells(&self, athlete: &CanonicalAthlete) -> Vec<Value> {
        let tally = self.expectations.tally(athlete.id.as_str());
        vec![
            Value::count(tally.map_or(0, |tally| tally.performances)),
            Value::count(tally.map_or(0, |tally| tally.meets.len())),
        ]
    }

    fn status_cells(&mut self, athlete: &CanonicalAthlete) -> Vec<Value> {
        let status = self
            .expectations
            .dataset
            .identities()
            .status(athlete.id.as_str());
        let Ok(status) = status else {
            return self.unverified_status(athlete);
        };
        vec![
            Value::text(status.as_str()),
            self.coverage_state(athlete),
            Value::flagged(status == IdentityStatus::RetainedConflict),
            Value::text(if status == IdentityStatus::Verified {
                "verified"
            } else {
                "review"
            }),
        ]
    }

    fn unverified_status(&mut self, athlete: &CanonicalAthlete) -> Vec<Value> {
        self.findings.note(format!(
            "athlete {} has no identity decision in the frozen archive, so its status, coverage, conflict and review columns are unverified",
            athlete.id.as_str()
        ));
        vec![
            Value::Empty,
            self.coverage_state(athlete),
            Value::Empty,
            Value::Empty,
        ]
    }

    fn coverage_state(&self, athlete: &CanonicalAthlete) -> Value {
        let performances = self
            .expectations
            .tally(athlete.id.as_str())
            .is_some_and(|tally| tally.performances > 0);
        let prs = self
            .expectations
            .prs_of(athlete.id.as_str())
            .next()
            .is_some();
        match (performances, prs) {
            (true, true) => Value::text("pr"),
            (true, false) => Value::text("performance"),
            (false, _) => Value::text("identity-only"),
        }
    }

    fn append_address(&mut self, values: &mut Vec<Value>, athlete: &CanonicalAthlete) {
        match self.expectations.school_address.get(athlete.id.as_str()) {
            Some(address) if !address.is_empty() => values.push(Value::text(address)),
            Some(_) => values.push(Value::Empty),
            None => {
                self.findings.note(format!(
                    "athlete {} has no frozen school address projection",
                    athlete.id
                ));
                values.push(Value::Empty);
            }
        }
    }
}

fn append_performance_cells(
    values: &mut Vec<Value>,
    athlete: &CanonicalAthlete,
    prs: &[&SharedSelection],
) {
    values.push(cells::flag(athlete.sports.iter().any(|sport| {
        matches!(sport, Sport::IndoorTrack | Sport::OutdoorTrack)
    })));
    values.push(cells::flag(athlete.sports.contains(&Sport::CrossCountry)));
    values.push(cells::flag(athlete.sports.contains(&Sport::IndoorTrack)));
    values.push(cells::flag(athlete.sports.contains(&Sport::OutdoorTrack)));
    values.push(cells::event_list(prs));
    values.push(cells::headline(prs));
    values.extend(cells::pr_events(prs));
}

fn append_reach(values: &mut Vec<Value>, reach: reach::Reach, athletics_url: Option<&str>) {
    values.push(reach.track_names);
    values.push(reach.track_emails);
    values.push(reach.cross_country_name);
    values.push(reach.cross_country_email);
    values.push(reach.professional);
    values.push(reach.director_name);
    values.push(reach.director_email);
    values.push(Value::optional(athletics_url));
    values.push(Value::Empty);
    values.push(Value::Empty);
    values.push(reach.all_emails);
    values.push(reach.preferred_name);
    values.push(reach.preferred_role);
    values.push(reach.preferred_email);
    values.push(reach.preferred_state);
    values.push(reach.preferred_coach_id);
    values.push(reach.preferred_source_url);
    values.push(reach.preferred_capture_sha256);
    values.push(reach.preferred_acquired_at);
}

fn append_profiles(values: &mut Vec<Value>, athlete: &CanonicalAthlete) {
    let profiles = cells::profiles_of(athlete);
    values.push(Value::optional(profiles.athletic_net.as_deref()));
    values.push(Value::optional(profiles.milesplit.as_deref()));
    values.push(Value::text(profiles.other.join("; ")));
    values.push(Value::count(cells::source_count(athlete)));
}
