use super::markup::after;
use super::research::{append, locator, purpose_outcome};
use super::{
    markup, CanonicalSchool, ContactResearchAttempt, FetchOutcome, Outcome, Purpose, SchoolYear,
};
use census_domain::model::{SchoolMailboxClaim, SourceRef};

pub(super) fn assess_offices(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    capture: &FetchOutcome,
    page: &markup::Page,
    link_count: usize,
) {
    for purpose in [Purpose::SchoolOffice, Purpose::AthleticsOffice] {
        let mut state = if page.limited
            || page.malformed
            || link_count >= crate::school_sites::MAX_SITE_PAGES
        {
            Outcome::Partial
        } else {
            Outcome::CompletedEmpty
        };
        for block in page
            .blocks
            .iter()
            .filter(|block| purpose_of(block) == Some(purpose))
        {
            state = state.combine(admit_block(school, capture, block));
        }
        if capture.fetched_at.get(..10).and_then(SchoolYear::from_date) != Some(year) {
            state = state.combine(Outcome::Stale);
        }
        let attempt = ContactResearchAttempt {
            locator: locator(capture).to_owned(),
            acquired_at: capture.fetched_at.clone(),
            source_sha256: Some(capture.content_digest.clone()),
            outcome: state,
            reason: "assessed page-bound office purpose".to_owned(),
        };
        let outcome = purpose_outcome(school, year, purpose, &attempt);
        append(
            school,
            year,
            purpose,
            ContactResearchAttempt { outcome, ..attempt },
        );
    }
}

fn admit_block(
    school: &mut CanonicalSchool,
    capture: &FetchOutcome,
    block: &markup::Block,
) -> Outcome {
    let Some(purpose) = purpose_of(block) else {
        return Outcome::CompletedEmpty;
    };
    if block.mailboxes.len() > 1 {
        return Outcome::Ambiguous;
    }
    let Some(mailbox) = block.mailboxes.first().cloned() else {
        return Outcome::CompletedEmpty;
    };
    let Some(year) = capture.fetched_at.get(..10).and_then(SchoolYear::from_date) else {
        return Outcome::Partial;
    };
    let claim = SchoolMailboxClaim {
        school: school.id.clone(),
        purpose,
        mailbox,
        school_year: year,
        source: SourceRef::new(
            crate::school_sites::SOURCE_ID,
            Some(locator(capture).to_owned()),
        ),
        source_sha256: capture.content_digest.clone(),
        acquired_at: capture.fetched_at.clone(),
        statement: crate::row_hygiene::clean_text(&block.text),
    };
    match claim.validate() {
        Ok(()) => {
            if !school.mailbox_claims.contains(&claim) {
                school.mailbox_claims.push(claim);
            }
            Outcome::CompletedClaims
        }
        Err(_) => Outcome::Partial,
    }
}

fn purpose_of(block: &markup::Block) -> Option<Purpose> {
    let text = block.text.trim();
    if after(
        text,
        "contact us anytime with questions or comments. email us at ",
    )
    .is_some()
    {
        return Some(Purpose::SchoolOffice);
    }
    for (label, purpose) in [
        ("school office", Purpose::SchoolOffice),
        ("main office", Purpose::SchoolOffice),
        ("athletics office", Purpose::AthleticsOffice),
        ("athletic office", Purpose::AthleticsOffice),
    ] {
        if let Some(rest) = after(text, label) {
            let rest =
                rest.trim_start_matches(|ch: char| ch.is_whitespace() || matches!(ch, ':' | '-'));
            if block
                .mailboxes
                .iter()
                .any(|mailbox| after(rest, mailbox).is_some())
            {
                return Some(purpose);
            }
        }
    }
    None
}
