//! The seam between the lane's roster and the state a finding is minted in.
//!
//! A case the lane can ask about has to start `Pending` or the lane never sees it, and a case the
//! census's own rules have already decided must not start `Pending` or it holds the seal open for work
//! nothing can do. Both halves are one list each, so the two are checked against each other here
//! rather than in either crate alone: `census-domain` owns the mint rule and this crate owns the
//! roster, and neither can see the other's list.

use census_domain::model::{
    ReviewCase, ReviewState, COHORT_DECISION_FAMILIES, WITHHELD_MAILBOX_FAMILY,
};

use super::ReviewFamily;

/// Every family the lane asks about is minted pending, and reads back as the family it was minted as.
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

/// The families the census's own rules decide are minted retained, and no lane asks about them: their
/// finding is published, not owed.
///
/// One list, two owners — the collection contract decides a withheld mailbox and the evidence rule
/// decides a cohort claim — so the test walks the domain's own names rather than restating them.
#[test]
fn the_families_the_census_decides_are_minted_retained_and_asked_about_by_nobody() {
    let decided = std::iter::once(WITHHELD_MAILBOX_FAMILY).chain(COHORT_DECISION_FAMILIES);
    for family in decided {
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
