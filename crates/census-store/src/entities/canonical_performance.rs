use super::{collision, record, union_vec, CanonicalPerformance, Entity, RetainedConflict};
use census_domain::model::{
    parse_result_status, Mark, ResultStatus, ResultStatusKind, SourceNamespace, TimingMethod,
};

impl Entity for CanonicalPerformance {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        let found = collision(self.id.as_str(), self, &other);
        let scalar_conflict = if found.is_none() {
            performance_context_conflict(self, &other)
                .or_else(|| performance_conflict(self, &other))
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
        if refines_mark(&self.mark, &other.mark)
            && self.event == other.event
            && self.team == other.team
        {
            self.mark = other.mark;
        }
        if self.adjustment.is_none()
            && other
                .adjustment
                .as_ref()
                .is_some_and(|adjustment| adjustment.adjusted_mark == self.mark)
        {
            self.adjustment = other.adjustment;
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
        if self.combined_completion.is_none() {
            self.combined_completion = other.combined_completion;
        }
        union_vec(&mut self.evidence, other.evidence);
    }
}

fn performance_conflict(
    left: &CanonicalPerformance,
    right: &CanonicalPerformance,
) -> Option<RetainedConflict> {
    if !is_performance_contradictory(left, right) {
        return None;
    }
    let mut statements = [
        performance_tuple_statement(left),
        performance_tuple_statement(right),
    ];
    statements.sort();
    Some(RetainedConflict::new(
        "Individual result scalar",
        left.id.as_str(),
        "mark, timing, place, observed_grade, adjustment, combined_completion",
        format!(
            "contradictory individual statements: {}",
            statements.join(" versus ")
        ),
    ))
}

fn is_performance_contradictory(left: &CanonicalPerformance, right: &CanonicalPerformance) -> bool {
    let timing =
        |value: Option<TimingMethod>| value.filter(|value| *value != TimingMethod::Unknown);
    contradictory_marks(&left.mark, &right.mark)
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
            .is_some_and(|(left, right)| left != right)
        || match (&left.adjustment, &right.adjustment) {
            (Some(left), Some(right)) => left != right,
            _ => false,
        }
        || match (&left.combined_completion, &right.combined_completion) {
            (Some(left), Some(right)) => left != right,
            _ => false,
        }
}

fn performance_tuple_statement(performance: &CanonicalPerformance) -> String {
    format!(
        "{:?}",
        (
            &performance.mark,
            performance.timing,
            performance.place,
            performance.observed_grade,
            &performance.adjustment,
            &performance.combined_completion
        )
    )
}

fn contradictory_marks(left: &Mark, right: &Mark) -> bool {
    match (left, right) {
        (Mark::Raw(_), Mark::Raw(_)) => false,
        (Mark::Raw(_), _) | (_, Mark::Raw(_)) => {
            explicit_terminal(left) || explicit_terminal(right)
        }
        _ => left != right,
    }
}

fn explicit_terminal(mark: &Mark) -> bool {
    match mark {
        Mark::Raw(raw) => matches!(
            parse_result_status(raw).map(ResultStatus::kind),
            Some(ResultStatusKind::ExplicitTerminal)
        ),
        _ => false,
    }
}

fn refines_mark(left: &Mark, right: &Mark) -> bool {
    matches!(left, Mark::Raw(_))
        && !explicit_terminal(left)
        && (!matches!(right, Mark::Raw(_)) || explicit_terminal(right))
}

fn source_owner(performance: &CanonicalPerformance) -> Option<(&SourceNamespace, &str)> {
    performance
        .source_athlete
        .as_ref()
        .map(|owner| (&owner.namespace, owner.id.as_str()))
}

fn performance_context_conflict(
    left: &CanonicalPerformance,
    right: &CanonicalPerformance,
) -> Option<RetainedConflict> {
    let owner = source_owner(left);
    if owner == source_owner(right)
        && !explicit_terminal(&left.mark)
        && !explicit_terminal(&right.mark)
    {
        return None;
    }
    if left.event == right.event
        && left.team == right.team
        && owner == source_owner(right)
        && (owner.is_some() || left.mark == right.mark)
    {
        return None;
    }
    let mut statements = [context_statement(left), context_statement(right)];
    statements.sort();
    Some(RetainedConflict::new(
        "Individual result context",
        left.id.as_str(),
        "event, team, source_athlete",
        format!(
            "unresolved or contradictory individual result contexts: {}",
            statements.join(" versus ")
        ),
    ))
}

fn context_statement(performance: &CanonicalPerformance) -> String {
    format!(
        "{:?}",
        (
            &performance.event,
            &performance.team,
            source_owner(performance),
        )
    )
}
