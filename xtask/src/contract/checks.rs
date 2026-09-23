//! The eight checks, in the order [`super::run`] prints them.
//!
//! A check that cannot be measured — a `cargo metadata` that fails, an unreadable file — is reported
//! as that check's own failure line rather than as a hard error, so one broken input cannot hide the
//! other six verdicts. Checks 1 and 3 read the declarations; 2 and 5 the registry; 4, 6 and 7 the
//! tree.

use anyhow::Result;
use census_domain::UsJurisdiction;
use std::collections::BTreeSet;

use super::registry;
use super::tree;
use super::Check;
use crate::retry;

/// The retry ceiling §9 and ADR-002 fix for a handler attribute.
const ATTRIBUTE_CEILING: u64 = 3;

/// The retry ceiling §9 and ADR-002 fix for an inner `RunRetryPolicy`.
const INNER_CEILING: u64 = 1;

/// How many jurisdictions `UsJurisdiction::ALL` models: the fifty states and the District of
/// Columbia.
const MODELLED: usize = 51;

/// Every check, numbered as the module documentation orders them.
pub(super) fn all() -> Vec<Check> {
    let table: [(usize, &'static str, fn() -> Result<Check>); 8] = [
        (1, "census scope", census_scope),
        (2, "athleticnet transport", registry::athleticnet_transport),
        (3, "retry ceilings", retry_ceilings),
        (4, "no python files", tree::python_free),
        (5, "source admission", registry::admissions),
        (6, "scanned package set", tree::package_parity),
        (7, "documented set", tree::documentation),
        (8, "adapter registration", registry::adapter_registration),
    ];
    table
        .into_iter()
        .map(|(number, name, check)| guarded(number, name, check))
        .collect()
}

/// Run one check, turning a hard error into that check's own failure line.
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

/// Check 1: the run scope is every jurisdiction the domain models except the two it excludes.
///
/// The expected list is derived from [`UsJurisdiction::ALL`] and
/// [`UsJurisdiction::EXCLUDED_FROM_CENSUS`], so the assertion reads as the rule it is ("the contiguous
/// market") rather than as a second list of forty-nine names to keep in step with the enum, and the
/// exclusion rule has one declaration in the tree instead of two.
///
/// The comparison is name-for-name and in order: a one-for-one substitution or a reordering keeps set
/// equality and still changes which jurisdictions a run covers, so only the exact list passes. A reader
/// who reorders the constant because "the set is the same" has changed the order every census identity
/// digest is taken in.
///
/// `UsJurisdiction::is_in_census_scope` is the gate every acquisition path asks, so it is checked
/// against the same constant: two spellings of one rule drift, and this is where that shows.
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

/// Check 3: every `max_attempts` site in production code is at or below its ceiling.
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

/// A jurisdiction list for a failure line, in the order the set held.
fn list(states: &[UsJurisdiction]) -> String {
    states.iter().map(spell).collect::<Vec<String>>().join(", ")
}

/// One jurisdiction as `<code> <name>`.
fn spell(state: &UsJurisdiction) -> String {
    format!("{} {}", state.code(), state.name())
}

/// `UsJurisdiction::ALL` without the jurisdictions `UsJurisdiction::EXCLUDED_FROM_CENSUS` names: the
/// list `CENSUS_SCOPE` has to reproduce exactly, in this order.
fn expected_scope() -> Vec<UsJurisdiction> {
    UsJurisdiction::ALL
        .iter()
        .copied()
        .filter(|state| !UsJurisdiction::EXCLUDED_FROM_CENSUS.contains(state))
        .collect()
}

/// The failure line for the first place `declared` and `expected` disagree, or `None` when they agree
/// name-for-name and in order.
///
/// The first difference is named rather than the two lists: a scope that was substituted or reordered
/// is one edit, and a reader who sees `index 12` looks at twelve rather than at forty-nine names. A
/// difference in length is reported as counts, because there is no element to point at.
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
            declared
                .get(index)
                .map(spell)
                .unwrap_or_else(|| "<nothing>".to_string()),
            expected
                .get(index)
                .map(spell)
                .unwrap_or_else(|| "<nothing>".to_string()),
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

    /// The declared constant is what the check measures, so the real list has to pass it.
    #[test]
    fn the_declared_scope_matches_expectations() {
        assert_eq!(
            scope_mismatch(&UsJurisdiction::CENSUS_SCOPE, &expected_scope()),
            None
        );
    }

    /// A one-for-one substitution keeps the length and the set size and still changes which
    /// jurisdiction a run covers: the case set equality cannot see.
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

    /// The same jurisdictions in another order are a different scope: the census identity digest is
    /// taken in this order.
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

    /// A list that stops early lost a jurisdiction: a length, not an element, so the line carries
    /// counts instead of an index.
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
