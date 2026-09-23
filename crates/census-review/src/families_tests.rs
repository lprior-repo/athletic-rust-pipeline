//! The seam between the lane's roster and the state a finding is minted in.
//!
//! A case the lane can ask about has to start `Pending` or the lane never sees it, and a case the
//! census's own rules have already decided must not start `Pending` or it holds the seal open for work
//! nothing can do. Both halves are one list each, so the two are checked against each other here
//! rather than in either crate alone: `census-domain` owns the mint rule and this crate owns the
//! roster, and neither can see the other's list.

use census_domain::model::{ReviewCase, ReviewState, WITHHELD_MAILBOX_FAMILY};

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
        assert_eq!(
            ReviewCase::minted(family.label(), "subject:1", "A Subject", "the finding").state,
            ReviewState::Pending,
            "{} is this lane's work, so it cannot be minted decided",
            family.label()
        );
    }
}

/// The one family the collection contract decides is minted retained, and no lane asks about it.
#[test]
fn the_contract_decided_family_is_minted_retained_and_asked_about_by_nobody() {
    assert_eq!(
        ReviewFamily::from_label(WITHHELD_MAILBOX_FAMILY),
        None,
        "a withheld mailbox has no evidence question for a reviewer to answer"
    );
    assert_eq!(
        ReviewCase::minted(
            WITHHELD_MAILBOX_FAMILY,
            "coach:1",
            "A Coach (Somewhere High)",
            "only a personal mailbox was published"
        )
        .state,
        ReviewState::Retained,
        "the row stays visible in the workbook's queues either way; the state says who decided it"
    );
}
