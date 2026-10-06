use anyhow::Result;
use census_domain::UsJurisdiction;
use std::collections::BTreeSet;

use super::reach;
use super::registry;
use super::tree;
use super::Check;
use crate::retry;

const ATTRIBUTE_CEILING: u64 = 3;

const INNER_CEILING: u64 = 1;

const MODELLED: usize = 51;

type Entry = (usize, &'static str, fn() -> Result<Check>);

pub(super) fn all() -> Vec<Check> {
    let table: [Entry; 9] = [
        (1, "census scope", census_scope),
        (2, "athleticnet transport", registry::athleticnet_transport),
        (3, "retry ceilings", retry_ceilings),
        (4, "no python files", tree::python_free),
        (5, "source admission", registry::admissions),
        (6, "scanned package set", tree::package_parity),
        (7, "documented set", tree::documentation),
        (8, "adapter registration", registry::adapter_registration),
        (9, "source reachability", reach::source_reachability),
    ];
    table
        .into_iter()
        .map(|(number, name, check)| guarded(number, name, check))
        .collect()
}

fn guarded(number: usize, name: &'static str, check: fn() -> Result<Check>) -> Check {
    match check() {
        Ok(check) => check,
        Err(error) => Check::violated(
            number,
            name,
            "the check could not be measured".to_string(),
            vec![format!("{error:#}")],
        ),
    }
}

fn census_scope() -> Result<Check> {
    const NAME: &str = "census scope";
    let declared: BTreeSet<UsJurisdiction> = UsJurisdiction::CENSUS_SCOPE.into_iter().collect();
    let mut failures: Vec<String> = Vec::new();
    if UsJurisdiction::ALL.len() != MODELLED {
        failures.push(format!(
            "UsJurisdiction::ALL models {} jurisdictions, not the {MODELLED} the scope rule is written against",
            UsJurisdiction::ALL.len()
        ));
    }
    if let Some(mismatch) = scope_mismatch(&UsJurisdiction::CENSUS_SCOPE, &expected_scope()) {
        failures.push(mismatch);
    }
    let admitted: BTreeSet<UsJurisdiction> = UsJurisdiction::ALL
        .into_iter()
        .filter(|state| state.is_in_census_scope())
        .collect();
    if admitted != declared {
        failures.push(format!(
            "is_in_census_scope admits {} jurisdictions and CENSUS_SCOPE declares {}; admitted but out of scope: [{}]; in scope but refused: [{}]",
            admitted.len(),
            declared.len(),
            list(&admitted.difference(&declared).copied().collect::<Vec<_>>()),
            list(&declared.difference(&admitted).copied().collect::<Vec<_>>()),
        ));
    }
    let detail = format!(
        "CENSUS_SCOPE is {} jurisdictions of the {MODELLED} modelled, in ALL order minus [{}]",
        declared.len(),
        UsJurisdiction::EXCLUDED_FROM_CENSUS
            .iter()
            .map(|state| state.code())
            .collect::<Vec<&str>>()
            .join(" ")
    );
    if failures.is_empty() {
        return Ok(Check::holds(1, NAME, detail));
    }
    Ok(Check::violated(1, NAME, detail, failures))
}

fn retry_ceilings() -> Result<Check> {
    const NAME: &str = "retry ceilings";
    let sites = retry::sites()?;
    let failures = retry::violations(&sites, ATTRIBUTE_CEILING, INNER_CEILING);
    let detail = format!(
        "{} handler attribute sites at most {ATTRIBUTE_CEILING}, {} inner policy sites at most {INNER_CEILING}, {} mentions neither grammar reads",
        sites.attribute.len(),
        sites.builder.len(),
        sites.unclaimed.len()
    );
    if failures.is_empty() {
        return Ok(Check::holds(3, NAME, detail));
    }
    Ok(Check::violated(3, NAME, detail, failures))
}

fn list(states: &[UsJurisdiction]) -> String {
    states.iter().map(spell).collect::<Vec<String>>().join(", ")
}

fn spell(state: &UsJurisdiction) -> String {
    format!("{} {}", state.code(), state.name())
}

fn expected_scope() -> Vec<UsJurisdiction> {
    UsJurisdiction::ALL
        .iter()
        .copied()
        .filter(|state| !UsJurisdiction::EXCLUDED_FROM_CENSUS.contains(state))
        .collect()
}

fn scope_mismatch(declared: &[UsJurisdiction], expected: &[UsJurisdiction]) -> Option<String> {
    if declared == expected {
        return None;
    }
    let first = declared
        .iter()
        .zip(expected)
        .position(|(found, want)| found != want);
    match first {
        Some(index) => Some(format!(
            "CENSUS_SCOPE must match UsJurisdiction::ALL minus EXCLUDED_FROM_CENSUS name-for-name, in \
             order; first difference at index {index}: {} where {} was expected",
            match declared.get(index) {
                Some(jurisdiction) => spell(jurisdiction),
                None => "<nothing>".to_string(),
            },
            match expected.get(index) {
                Some(jurisdiction) => spell(jurisdiction),
                None => "<nothing>".to_string(),
            },
        )),
        None => Some(format!(
            "CENSUS_SCOPE must match UsJurisdiction::ALL minus EXCLUDED_FROM_CENSUS name-for-name, in \
             order; it holds {} jurisdictions where {} were expected",
            declared.len(),
            expected.len()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{expected_scope, scope_mismatch};
    use census_domain::UsJurisdiction;

    #[test]
    fn the_declared_scope_matches_expectations() {
        assert_eq!(
            scope_mismatch(&UsJurisdiction::CENSUS_SCOPE, &expected_scope()),
            None
        );
    }

    #[test]
    fn a_substitution_is_reported_at_its_index() {
        let expected = expected_scope();
        let mut declared = expected.clone();
        if let Some(slot) = declared.first_mut() {
            *slot = UsJurisdiction::Alaska;
        }
        let line = scope_mismatch(&declared, &expected);
        assert!(
            line.as_deref().is_some_and(|text| text.contains("index 0")),
            "{line:?}"
        );
    }

    #[test]
    fn a_reordering_is_reported() {
        let expected = expected_scope();
        let mut declared = expected.clone();
        declared.reverse();
        let line = scope_mismatch(&declared, &expected);
        assert!(
            line.as_deref().is_some_and(|text| text.contains("index 0")),
            "{line:?}"
        );
    }

    #[test]
    fn a_short_list_is_reported_as_counts() {
        let expected = expected_scope();
        let mut declared = expected.clone();
        declared.truncate(declared.len().saturating_sub(1));
        let line = scope_mismatch(&declared, &expected);
        assert!(
            line.as_deref()
                .is_some_and(|text| text.contains("jurisdictions where")),
            "{line:?}"
        );
    }
}
