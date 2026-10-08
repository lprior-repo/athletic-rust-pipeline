mod markup;

use crate::net::FetchOutcome;
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::{
    normalize_name, CanonicalSchool, ContactResearch, ContactResearchAttempt,
    ContactResearchOutcome as Outcome, ContactResearchSubject as Subject, SchoolMailboxClaim,
    SchoolMailboxPurpose as Purpose, SchoolYear, SourceRef,
};
use sha2::{Digest, Sha256};

#[tracing::instrument(skip(ctx, school))]
pub async fn research_school_mailboxes(
    ctx: &AdapterContext<'_>,
    school: &mut CanonicalSchool,
) -> CrawlResult<()> {
    admit_retained(school)?;
    let checkpoint = (school.mailbox_claims.len(), school.contact_research.len());
    let Some(site) = school.school_website.as_deref().map(str::to_owned) else {
        unattempted(school, ctx.school_year);
        crate::school_sites::persist_contact_prefix(ctx, school, checkpoint)?;
        return Ok(());
    };
    let links = fetch(ctx, school, &site).await?;
    for url in links
        .into_iter()
        .take(crate::school_sites::MAX_SITE_PAGES.saturating_sub(1))
    {
        fetch(ctx, school, &url).await?;
    }
    Ok(())
}

#[tracing::instrument(skip(ctx, school))]
async fn fetch(
    ctx: &AdapterContext<'_>,
    school: &mut CanonicalSchool,
    url: &str,
) -> CrawlResult<Vec<String>> {
    let checkpoint = (school.mailbox_claims.len(), school.contact_research.len());
    if let Err(error) = admit_retained(school) {
        if school.contact_research.len() > 65_527 {
            return Err(error);
        }
        let attempt = ContactResearchAttempt {
            locator: url.to_owned(),
            acquired_at: crate::net::now_iso8601(),
            source_sha256: None,
            outcome: Outcome::Partial,
            reason: error.to_string(),
        };
        crate::school_sites::retain_contact_attempt(school, ctx.school_year, attempt.clone());
        record(school, ctx.school_year, attempt);
        crate::school_sites::persist_contact_prefix(ctx, school, checkpoint)?;
        return Ok(Vec::new());
    }
    let outcome = match ctx.fetcher.get(url, &ctx.fetch_options()).await {
        Ok(outcome) => outcome,
        Err(error) => {
            let attempt = ContactResearchAttempt {
                locator: url.to_owned(),
                acquired_at: crate::net::now_iso8601(),
                source_sha256: None,
                outcome: super::research_failure::fetch(&error),
                reason: error.to_string(),
            };
            crate::school_sites::retain_contact_attempt(school, ctx.school_year, attempt.clone());
            record(school, ctx.school_year, attempt);
            crate::school_sites::persist_contact_prefix(ctx, school, checkpoint)?;
            return Ok(Vec::new());
        }
    };
    let links = crate::school_sites::apply_contact_capture(school, ctx.school_year, &outcome)?;
    crate::school_sites::persist_contact_prefix(ctx, school, checkpoint)?;
    Ok(links)
}

pub fn apply_school_mailbox_capture(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    capture: &FetchOutcome,
) -> CrawlResult<Vec<String>> {
    admit_retained(school)?;
    if chrono::DateTime::parse_from_rfc3339(&capture.fetched_at).is_err() {
        record_capture(
            school,
            year,
            capture,
            Outcome::Failed,
            "school contact capture time is missing or invalid".to_owned(),
        );
        return Ok(Vec::new());
    }
    if capture.body.len() > 1024 * 1024 {
        record_capture(
            school,
            year,
            capture,
            Outcome::Partial,
            "school contact body exceeds 1 MiB".to_owned(),
        );
        return Ok(Vec::new());
    }
    let digest = format!("{:x}", Sha256::digest(&capture.body));
    if digest != capture.content_digest {
        return Err(schema(
            "generic contact capture digest does not match retained bytes",
        ));
    }
    if !(200..300).contains(&capture.status) {
        record_capture(
            school,
            year,
            capture,
            super::research_failure::status(capture.status),
            format!("official contact page returned HTTP {}", capture.status),
        );
        return Ok(Vec::new());
    }
    let parsed = markup::parse(&capture.body);
    let page = match parsed {
        Ok(page) => page,
        Err(error) => {
            record_capture(school, year, capture, error.outcome(), error.to_string());
            return Ok(Vec::new());
        }
    };
    admit_page(school, year, capture, page)
}

fn admit_page(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    capture: &FetchOutcome,
    page: markup::Page,
) -> CrawlResult<Vec<String>> {
    let links = match contact_links(school, capture, &page.links) {
        Ok(links) => links,
        Err(error) => {
            record_capture(school, year, capture, Outcome::Ambiguous, error.to_string());
            return Ok(Vec::new());
        }
    };
    if page.ambiguous
        || !page
            .owner
            .as_deref()
            .is_some_and(|owner| normalize_name(owner) == school.normalized_name)
    {
        record_capture(
            school,
            year,
            capture,
            Outcome::Ambiguous,
            "published page owner does not uniquely match the canonical school".to_owned(),
        );
        return Ok(links);
    }
    assess_offices(school, year, capture, &page, links.len());
    Ok(links)
}

fn assess_offices(
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

fn after<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if !text.get(..prefix.len())?.eq_ignore_ascii_case(prefix) {
        return None;
    }
    text.get(prefix.len()..)
}

fn conflicting(school: &CanonicalSchool, year: SchoolYear, purpose: Purpose) -> bool {
    let mut claims = school.mailbox_claims.iter().filter(|claim| {
        claim.school_year == year
            && claim.purpose == purpose
            && claim.acquired_at.get(..10).and_then(SchoolYear::from_date) == Some(year)
    });
    let Some(first) = claims.next() else {
        return false;
    };
    claims.any(|claim| {
        !first
            .mailbox
            .trim()
            .eq_ignore_ascii_case(claim.mailbox.trim())
    })
}

fn contact_links(
    school: &CanonicalSchool,
    capture: &FetchOutcome,
    links: &[String],
) -> CrawlResult<Vec<String>> {
    let site = school
        .school_website
        .as_deref()
        .ok_or_else(|| schema("school mailbox acquisition has no official website"))?;
    let site =
        url::Url::parse(site).map_err(|_| schema("official school website is not a parsed URL"))?;
    let page = url::Url::parse(
        capture
            .response_url
            .as_deref()
            .map_or(capture.url.as_str(), |url| url),
    )
    .map_err(|_| schema("school contact capture has invalid URL"))?;
    if page.origin() != site.origin() {
        return Err(schema(
            "school mailbox capture left its authorized official school origin",
        ));
    }
    let mut urls = std::collections::BTreeSet::new();
    for href in links {
        let resolved = page
            .join(href)
            .map_err(|_| schema("published school contact link is malformed"))?;
        if resolved.origin() == site.origin() && resolved != site && resolved != page {
            urls.insert(resolved.to_string());
        }
    }
    Ok(urls.into_iter().collect())
}

fn record_capture(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    capture: &FetchOutcome,
    outcome: Outcome,
    reason: String,
) {
    record(
        school,
        year,
        ContactResearchAttempt {
            locator: locator(capture).to_owned(),
            acquired_at: capture.fetched_at.clone(),
            source_sha256: Some(capture.content_digest.clone()),
            outcome,
            reason,
        },
    );
}

fn locator(capture: &FetchOutcome) -> &str {
    capture
        .response_url
        .as_deref()
        .map_or(capture.url.as_str(), |url| url)
}

fn record(school: &mut CanonicalSchool, year: SchoolYear, attempt: ContactResearchAttempt) {
    let office = purpose_outcome(school, year, Purpose::SchoolOffice, &attempt);
    let athletics = purpose_outcome(school, year, Purpose::AthleticsOffice, &attempt);
    let mut office_attempt = attempt.clone();
    office_attempt.outcome = office;
    append(school, year, Purpose::SchoolOffice, office_attempt);
    let mut athletics_attempt = attempt;
    athletics_attempt.outcome = athletics;
    append(school, year, Purpose::AthleticsOffice, athletics_attempt);
}

fn append(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    purpose: Purpose,
    attempt: ContactResearchAttempt,
) {
    let research = ContactResearch {
        school: school.id.clone(),
        subject: Subject::SchoolMailbox(purpose),
        school_year: year,
        outcome: attempt.outcome.clone(),
        attempts: vec![attempt],
    };
    if !school.contact_research.contains(&research) {
        school.contact_research.push(research);
    }
}

fn purpose_outcome(
    school: &CanonicalSchool,
    year: SchoolYear,
    purpose: Purpose,
    attempt: &ContactResearchAttempt,
) -> Outcome {
    if !attempt.outcome.is_terminal() {
        return attempt.outcome.clone();
    }
    if conflicting(school, year, purpose) {
        return Outcome::Conflict;
    }
    if school.mailbox_claims.iter().any(|claim| {
        claim.purpose == purpose
            && claim.school_year == year
            && claim.source.url.as_deref() == Some(attempt.locator.as_str())
            && Some(claim.source_sha256.as_str()) == attempt.source_sha256.as_deref()
            && claim.acquired_at == attempt.acquired_at
    }) {
        Outcome::CompletedClaims
    } else {
        Outcome::CompletedEmpty
    }
}

fn schema(detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: crate::school_sites::SOURCE_ID.to_owned(),
        detail: detail.to_owned(),
    }
}

fn admit_retained(school: &CanonicalSchool) -> CrawlResult<()> {
    for (resource, requested, limit) in [
        (
            "school mailbox claims",
            school.mailbox_claims.len(),
            65_536usize,
        ),
        (
            "school contact research rows",
            school.contact_research.len(),
            65_536usize,
        ),
    ] {
        if requested > limit.saturating_sub(4096) {
            return Err(CrawlError::Resource {
                resource,
                requested,
                limit,
            });
        }
    }
    Ok(())
}

fn unattempted(school: &mut CanonicalSchool, year: SchoolYear) {
    for purpose in [Purpose::SchoolOffice, Purpose::AthleticsOffice] {
        let research = ContactResearch {
            school: school.id.clone(),
            subject: Subject::SchoolMailbox(purpose),
            school_year: year,
            outcome: Outcome::Unattempted,
            attempts: Vec::new(),
        };
        if !school.contact_research.contains(&research) {
            school.contact_research.push(research);
        }
    }
}

#[cfg(test)]
mod publication_tests;
#[cfg(test)]
mod tests;

pub(crate) fn inspect_staff_links(body: &[u8]) -> CrawlResult<(bool, Vec<String>)> {
    let page = markup::parse(body).map_err(|error| schema(&error.to_string()))?;
    if page.limited {
        return Err(CrawlError::Resource {
            resource: "published staff discovery page",
            requested: 65_537,
            limit: 65_536,
        });
    }
    if page.malformed {
        return Err(schema("published staff discovery page is malformed"));
    }
    Ok((page.sidearm, page.links))
}
