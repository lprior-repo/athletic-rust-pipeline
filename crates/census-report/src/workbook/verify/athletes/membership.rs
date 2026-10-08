use super::super::report::Findings;
use census_domain::model::CanonicalAthlete;
use std::collections::BTreeMap;

pub(super) fn verify(ordered: &[&CanonicalAthlete], printed: &[String], findings: &mut Findings) {
    let expected = count_ids(ordered.iter().map(|athlete| athlete.id.as_str()));
    let actual = count_ids(printed.iter().map(String::as_str));
    report_excess(&expected, &actual, findings);
    report_missing(&expected, &actual, findings);
}

fn count_ids<'a>(ids: impl Iterator<Item = &'a str>) -> BTreeMap<&'a str, usize> {
    let mut counts = BTreeMap::<&str, usize>::new();
    for id in ids {
        let count = counts.entry(id).or_insert(0);
        *count = count.saturating_add(1);
    }
    counts
}

fn report_excess(
    expected: &BTreeMap<&str, usize>,
    actual: &BTreeMap<&str, usize>,
    findings: &mut Findings,
) {
    for (id, count) in actual {
        let held = expected.get(id).copied().map_or(0, core::convert::identity);
        if held < *count {
            let message = excess_message(id, *count, held, expected.contains_key(id));
            findings.note(message);
        }
    }
}

fn excess_message(id: &str, count: usize, held: usize, known: bool) -> String {
    if known {
        format!(
            "athlete {id} is printed {count} times where the frozen cohort holds it {held} time(s)"
        )
    } else {
        format!("athlete {id} is printed but absent from the frozen cohort")
    }
}

fn report_missing(
    expected: &BTreeMap<&str, usize>,
    actual: &BTreeMap<&str, usize>,
    findings: &mut Findings,
) {
    for (id, count) in expected {
        let found = actual.get(id).copied().map_or(0, core::convert::identity);
        if found < *count {
            findings.note(format!(
                "athlete {id} holds {count} cohort row(s) in the frozen dataset but is printed {found} time(s)"
            ));
        }
    }
}
