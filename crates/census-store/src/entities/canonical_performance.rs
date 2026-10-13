use super::{collision, record, union_vec, CanonicalPerformance, Entity, RetainedConflict};
use census_domain::model::{Mark, TimingMethod};

impl Entity for CanonicalPerformance {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        let found = collision(self.id.as_str(), self, &other);
        let scalar_conflict = if found.is_none() {
            performance_conflict(self, &other)
        } else {
            None
        };
        other
            .retained_conflicts
            .into_iter()
            .for_each(|conflict| record(&mut self.retained_conflicts, conflict));
        if let Some(conflict) = found {
            record(&mut self.retained_conflicts, conflict);
            return;
        }
        if let Some(conflict) = scalar_conflict {
            record(&mut self.retained_conflicts, conflict);
            union_vec(&mut self.evidence, other.evidence);
            return;
        }
        if matches!(self.mark, Mark::Raw(_))
            && !matches!(other.mark, Mark::Raw(_))
            && self.event == other.event
            && self.team == other.team
        {
            self.mark = other.mark;
        }
        if self.wind_mps.is_none() {
            self.wind_mps = other.wind_mps;
        }
        if self.place.is_none() {
            self.place = other.place;
        }
        if self.observed_grade.is_none() {
            self.observed_grade = other.observed_grade;
        }
        if matches!(self.timing, None | Some(TimingMethod::Unknown))
            && other.timing.is_some()
            && self.event == other.event
            && self.team == other.team
        {
            self.timing = other.timing;
        }
        if self.source_athlete.is_none() {
            self.source_athlete = other.source_athlete;
        }
        union_vec(&mut self.evidence, other.evidence);
    }
}

fn performance_conflict(
    left: &CanonicalPerformance,
    right: &CanonicalPerformance,
) -> Option<RetainedConflict> {
    let timing =
        |value: Option<TimingMethod>| value.filter(|value| *value != TimingMethod::Unknown);
    let contradictory = (left.mark != right.mark
        && !matches!(left.mark, Mark::Raw(_))
        && !matches!(right.mark, Mark::Raw(_)))
        || timing(left.timing)
            .zip(timing(right.timing))
            .is_some_and(|(left, right)| left != right)
        || left
            .place
            .zip(right.place)
            .is_some_and(|(left, right)| left != right)
        || left
            .observed_grade
            .zip(right.observed_grade)
            .is_some_and(|(left, right)| left != right);
    if !contradictory {
        return None;
    }
    let mut statements = [
        format!(
            "{:?}",
            (&left.mark, left.timing, left.place, left.observed_grade)
        ),
        format!(
            "{:?}",
            (&right.mark, right.timing, right.place, right.observed_grade)
        ),
    ];
    statements.sort();
    Some(RetainedConflict::new(
        "Individual result scalar",
        left.id.as_str(),
        "mark, timing, place, observed_grade",
        format!(
            "contradictory individual statements: {}",
            statements.join(" versus ")
        ),
    ))
}
