
use census_domain::model::{ReviewCase, ReviewState, COHORT_DECISION_FAMILIES};

use super::ReviewFamily;

#[test]
fn an_askable_family_is_minted_pending_and_names_itself() {
    for family in ReviewFamily::askable() {
        assert_eq!(
            ReviewFamily::from_label(family.label()),
            Some(family),
            "the label {} has to name the family that asks for it",
            family.label()
        );
        assert!(
            !family.field().is_empty(),
            "{} asks for a field, so the packet has something to answer",
            family.label()
        );
        assert!(
            !ReviewCase::decided_by_its_own_rules(family.label()),
            "{} is this lane's work, so the census's rules cannot have decided it already",
            family.label()
        );
        assert_eq!(
            ReviewCase::minted(family.label(), "subject:1", "A Subject", "the finding").state,
            ReviewState::Pending,
            "{} is this lane's work, so it cannot be minted decided",
            family.label()
        );
    }
}

#[test]
fn the_families_the_census_decides_are_minted_retained_and_asked_about_by_nobody() {
    for family in COHORT_DECISION_FAMILIES {
        assert!(
            ReviewCase::decided_by_its_own_rules(family),
            "{family} is retired by the rule that mints it"
        );
        assert_eq!(
            ReviewFamily::from_label(family),
            None,
            "{family} has no evidence question for a reviewer to answer"
        );
        assert_eq!(
            ReviewCase::minted(family, "subject:1", "A Subject", "the finding").state,
            ReviewState::Retained,
            "the row stays visible in the workbook's queues either way; the state says who decided it"
        );
    }
}
