use super::{
    CoverageTotals, JurisdictionBucket, Outcome, UNKNOWN_JURISDICTION, totals_text,
};
use crate::export::ExportDataset;

pub(super) fn coverage_notes(
    dataset: &ExportDataset,
    grad_year: Option<i16>,
    outcome: &Outcome,
) -> Vec<String> {
    let cohort = grad_year.map_or_else(|| "all".to_string(), |year| year.to_string());
    let mut notes = vec![format!(
        "coverage grad_year={cohort} athletes={} jurisdictions={} from {}",
        outcome.read.athletes,
        outcome.jurisdictions.len(),
        dataset.lineage.store_root
    )];
    append_population_notes(&mut notes, outcome, &cohort);
    append_placement_notes(&mut notes, outcome);
    append_lifecycle_notes(&mut notes, outcome);
    notes
}

fn append_population_notes(notes: &mut Vec<String>, outcome: &Outcome, cohort: &str) {
    if outcome.read.athletes == 0 {
        notes.push(
            "no cohort athletes stored yet — run `collect` then `consolidate` before `report`"
                .to_string(),
        );
    }
    if outcome.off_cohort_athletes > 0 {
        notes.push(format!(
            "{} stored athletes are outside grad_year={cohort} and publish as off_cohort_athletes",
            outcome.off_cohort_athletes
        ));
    }
}

fn append_placement_notes(notes: &mut Vec<String>, outcome: &Outcome) {
    if outcome.outside_scope != CoverageTotals::default() {
        notes.push(format!(
            "stored rows outside the census run scope (the 48 continental states plus DC, ADR-009) \
             are excluded from `coverage read` and population coverage rows; retained historical \
             school/meet metadata may appear in selected athletes' results: {}",
            totals_text(&outcome.outside_scope)
        ));
    }
    let unplaceable = outcome
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction == JurisdictionBucket::Unplaced)
        .map_or(0, |row| row.athletes);
    if unplaceable > 0 {
        notes.push(format!(
            "{unplaceable} cohort athletes carry no placeable jurisdiction and publish in the {UNKNOWN_JURISDICTION} row"
        ));
    }
}

fn append_lifecycle_notes(notes: &mut Vec<String>, outcome: &Outcome) {
    let population = &outcome.source_population;
    if let Some(registered) = population.registered {
        notes.push(format!(
            "source lifecycle: {registered} registered -> {} -> {} attempted -> {} captured -> {} parsed -> {} accepted -> {} withheld",
            stage_text(population.applicable),
            stage_text(population.attempted),
            stage_text(population.captured),
            stage_text(population.parsed),
            stage_text(population.accepted),
            stage_text(population.withheld)
        ));
    } else {
        notes.push(
            "source lifecycle: unmeasured — the run has published no CoverageRow population for this store"
                .to_string(),
        );
    }

    let contact = &outcome.contact;
    let contact_total = contact.attempted.saturating_add(contact.never_attempted);
    if contact_total > 0 {
        notes.push(format!(
            "contact attempts: {} attempted ({} reached, {} failed, {} blocked, {} successful-empty, {} accepted refusal, {} exhausted) + {} never attempted",
            contact.attempted,
            contact.reached,
            contact.failed,
            contact.blocked,
            contact.successful_empty,
            contact.accepted_refusal,
            contact.exhausted,
            contact.never_attempted
        ));
    }

    if outcome.raw_observations > 0 {
        notes.push(format!(
            "coverage denominators: {} raw observations -> {} scope-filtered -> {} accepted cohort",
            outcome.raw_observations, outcome.scope_filtered, outcome.accepted_cohort
        ));
    }

    if !outcome.terminal_failures.is_empty() {
        let failures: Vec<String> = outcome
            .terminal_failures
            .iter()
            .map(|(reason, count)| format!("{} ({})", reason, count))
            .collect();
        notes.push(format!("terminal failures: {}", failures.join(", ")));
    }
}

fn stage_text(stage: Option<usize>) -> String {
    stage.map_or_else(|| "unmeasured".to_string(), |value| value.to_string())
}
