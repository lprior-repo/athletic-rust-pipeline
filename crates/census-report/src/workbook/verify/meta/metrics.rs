use crate::export::ExportDataset;
use crate::report::{Census, Derivation, ReportResult, Scope};
use crate::workbook::meta::metrics::counters::{
    count_athletic_net_meets as athletic_net_meets, count_co2027 as co2027,
    count_coaches_with_email as coaches_with_email, count_grade_evidence as grade_evidence,
};
use crate::workbook::meta::Family;
use crate::workbook::Censuses;
use census_domain::model::{SchoolYear, SCHOOL_IDENTITY_FAMILY};

use super::{header, number_row, text_row, Expect, Series, Sheet};

const HEADERS: [&str; 2] = ["Run metric", "Value"];

const FROZEN_HEADERS: [&str; 2] = ["Frozen input", "Value"];

const SCOPE_HEADERS: [&str; 4] = ["Core", "Count", "All sources", "Count"];

const NOTE_HEADERS: [&str; 2] = ["Method note", "Value"];

const RECONCILE_HEADERS: [&str; 4] = ["Reconciled counter", "Sheet rows", "Census", "Status"];

const RECONCILED: [&str; 9] = [
    "Schools",
    "Meets",
    "Athletes",
    "Coaches",
    "Coaches with a published email",
    "Class-of-2027 athletes",
    "Class-of-2027 athletes with grade evidence",
    "Meets naming an Athletic.net id",
    "Schools sharing a normalized name",
];

type FrozenCount = (&'static str, fn(&ExportDataset) -> usize);
type ScopeCounter = (&'static str, fn(&Census) -> usize);

const FROZEN_COUNTS: [FrozenCount; 12] = [
    ("athletes", |dataset| dataset.athletes.len()),
    ("schools", |dataset| dataset.schools.len()),
    ("teams", |dataset| dataset.teams.len()),
    ("coaches", |dataset| dataset.coaches.len()),
    ("coach observations", |dataset| {
        dataset.coach_observations.len()
    }),
    ("events", |dataset| dataset.events.len()),
    ("meets", |dataset| dataset.meets.len()),
    ("performances", |dataset| dataset.performances.len()),
    ("review cases", |dataset| dataset.review_cases.len()),
    ("source access", |dataset| dataset.source_access.len()),
    ("identity verdicts", |dataset| dataset.verdicts.len()),
    ("identity decisions", |dataset| {
        dataset.identity_decisions.len()
    }),
];

const SCOPE_COUNTERS: [ScopeCounter; 7] = [
    ("Schools", |census| census.totals.schools),
    ("Athletes", |census| census.totals.athletes),
    ("Coaches", |census| census.totals.coaches),
    ("Coaches with a published email", |census| {
        census.totals.coaches_with_email
    }),
    ("Class of 2027", |census| census.totals.class_of_2027),
    ("Meets", |census| census.meets.total),
    ("Meets naming an Athletic.net id", |census| {
        census.meets.with_athletic_net_id
    }),
];

pub(super) fn expected(series: &Series<'_, '_>, conflicts: &[Family]) -> ReportResult<Sheet> {
    let mut rows = vec![header(&HEADERS)];
    rows.extend(top_rows(
        series.dataset,
        series.recruiting,
        series.school_year,
        series.bests,
    )?);
    rows.push(Vec::new());
    rows.extend(frozen_rows(series.dataset)?);
    rows.push(Vec::new());
    rows.extend(scope_rows(series.censuses)?);
    rows.push(Vec::new());
    rows.extend(note_rows(&series.censuses.core));
    rows.push(Vec::new());
    rows.extend(reconcile_rows(
        series.scope,
        series.population,
        series.censuses,
        conflicts,
    )?);
    Ok(("Run Metrics", rows))
}

fn top_rows(
    dataset: &ExportDataset,
    recruiting: &Derivation<'_>,
    school_year: SchoolYear,
    bests: usize,
) -> ReportResult<Vec<Vec<Expect>>> {
    let lineage = &dataset.lineage;
    Ok(vec![
        text_row("Workbook generated on", &lineage.generated_on),
        text_row("Store", &lineage.store_root),
        text_row("Workbook scope", recruiting.scope().as_str()),
        text_row("Contact assessment school year", &school_year.short()),
        text_row("Census scopes published", "core + all sources"),
        vec![
            Expect::text("Recruiting athletes"),
            Expect::count(recruiting.athletes().len())?,
        ],
        vec![
            Expect::text("Best-mark rows reduced"),
            Expect::count(bests)?,
        ],
        vec![
            Expect::text(crate::workbook::recruiting::coach_spelling::SHEET_ROWS),
            Expect::count(coach_census(recruiting).rows)?,
        ],
        vec![
            Expect::text(crate::workbook::recruiting::coach_spelling::DISTINCT_SCHOOLS),
            Expect::count(coach_census(recruiting).schools)?,
        ],
        vec![
            Expect::text(crate::workbook::recruiting::coach_spelling::DISTINCT_COACH_IDS),
            Expect::count(coach_census(recruiting).coach_ids)?,
        ],
        text_row("Cohort behind the counters", "class of 2027"),
    ])
}

fn coach_census(
    recruiting: &Derivation<'_>,
) -> crate::workbook::recruiting::coach_spelling::CoachSheetCensus {
    crate::workbook::recruiting::coach_spelling::coach_sheet_census(recruiting.coach_observations())
}

fn frozen_rows(dataset: &ExportDataset) -> ReportResult<Vec<Vec<Expect>>> {
    let lineage = &dataset.lineage;
    let mut rows = vec![
        header(&FROZEN_HEADERS),
        text_row("Store identity", &lineage.store_identity),
        text_row("Input generation", &lineage.input_generation),
        text_row("Input content digest", &lineage.input_digest),
        text_row("Source content digest", &lineage.source_digest),
        text_row(
            "Captured store sequence",
            &lineage.snapshot_sequence.to_string(),
        ),
        number_row("Export schema revision", f64::from(lineage.schema_revision)),
        number_row("Export policy revision", f64::from(lineage.policy_revision)),
        text_row("Frozen table", "Canonical records"),
    ];
    for (table, read) in FROZEN_COUNTS {
        rows.push(vec![Expect::text(table), Expect::count(read(dataset))?]);
    }
    Ok(rows)
}

fn scope_rows(censuses: &Censuses) -> ReportResult<Vec<Vec<Expect>>> {
    let mut rows = vec![header(&SCOPE_HEADERS)];
    for (label, read) in SCOPE_COUNTERS {
        rows.push(vec![
            Expect::text(label),
            Expect::count(read(&censuses.core))?,
            Expect::text(label),
            Expect::count(read(&censuses.all_sources))?,
        ]);
    }
    Ok(rows)
}

fn note_rows(core: &Census) -> Vec<Vec<Expect>> {
    let mut rows = vec![header(&NOTE_HEADERS)];
    for note in core
        .notes
        .iter()
        .filter(|note| note.starts_with("core performance publication"))
    {
        rows.push(vec![
            Expect::text("Core performance publication"),
            Expect::text(note.as_str()),
        ]);
    }
    rows
}

fn reconcile_rows(
    scope: Scope,
    population: &Derivation<'_>,
    censuses: &Censuses,
    conflicts: &[Family],
) -> ReportResult<Vec<Vec<Expect>>> {
    let census = match scope {
        Scope::Core => &censuses.core,
        Scope::AllSources => &censuses.all_sources,
    };
    let sheets = sheet_counts(population, conflicts);
    let published = census_counts(census);
    let mut rows = vec![header(&RECONCILE_HEADERS)];
    for ((label, sheet), count) in RECONCILED.iter().copied().zip(sheets).zip(published) {
        rows.push(reconciled_row(label, sheet, count)?);
    }
    Ok(rows)
}

fn sheet_counts(population: &Derivation<'_>, conflicts: &[Family]) -> [usize; 9] {
    let athletes = population.athletes();
    let meets = population.meets();
    let coaches = population.coaches();
    [
        population.schools().len(),
        meets.len(),
        athletes.len(),
        coaches.len(),
        coaches_with_email(coaches),
        co2027(athletes),
        grade_evidence(athletes),
        athletic_net_meets(meets),
        findings_of(conflicts, SCHOOL_IDENTITY_FAMILY),
    ]
}

fn census_counts(census: &Census) -> [usize; 9] {
    let totals = &census.totals;
    [
        totals.schools,
        census.meets.total,
        totals.athletes,
        totals.coaches,
        totals.coaches_with_email,
        totals.class_of_2027,
        totals.class_of_2027_with_grad_year_evidence,
        census.meets.with_athletic_net_id,
        census.duplicate_school_names,
    ]
}

fn reconciled_row(label: &str, sheet: usize, published: usize) -> ReportResult<Vec<Expect>> {
    Ok(vec![
        Expect::text(label),
        Expect::count(sheet)?,
        Expect::count(published)?,
        Expect::text(status(sheet, published)),
    ])
}

fn findings_of(families: &[Family], label: &str) -> usize {
    families
        .iter()
        .find(|family| family.label == label)
        .map_or(0, |family| family.findings)
}

fn status(sheet: usize, published: usize) -> &'static str {
    if sheet == published {
        "reconciled"
    } else {
        "DIFFERS"
    }
}
