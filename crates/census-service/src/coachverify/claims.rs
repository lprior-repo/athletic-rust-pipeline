//! Field-level evidence from one bounded staff record; never concatenate different staff cards.
use super::{normalize, FragmentRow};
use scraper::{Html, Selector};
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_SPAN: usize = 4096;

#[derive(Debug, Clone, Serialize)]
pub struct ClaimEvidence {
    pub field: String,
    pub value: String,
    pub person: String,
    pub role: String,
    pub sport: String,
    pub school: String,
    pub state: String,
    pub source_url: String,
    pub retrieved_at: String,
    pub claimed_observed_on: String,
    pub source_sha256: String,
    pub span: String,
}

pub(super) struct PageClaims {
    pub fields: Vec<ClaimEvidence>,
    pub found: bool,
    pub contradicted: bool,
}

struct ClaimContext<'a> {
    row: &'a FragmentRow,
    url: &'a str,
    retrieved_at: &'a str,
    digest: &'a str,
}

pub(super) fn inspect(
    text: &str, row: &FragmentRow, url: &str, retrieved_at: &str,
) -> anyhow::Result<PageClaims> {
    let (spans, heading) = spans(text)?;
    let school = normalize(&row.school);
    let digest = format!("{:x}", Sha256::digest(text.as_bytes()));
    let context = ClaimContext { row, url, retrieved_at, digest: &digest };
    let mut result = PageClaims { fields: Vec::new(), found: false, contradicted: false };
    spans.iter().filter(|span| span.len() <= MAX_SPAN).for_each(|span| {
        let flat = normalize(span);
        [false, true].into_iter().for_each(|director| {
            let (person, email) = if director { (&row.ad_name, &row.ad_email) }
                else { (&row.coach_name, &row.public_professional_email) };
            if person.is_empty() || !contains(&flat, &normalize(person)) { return; }
            result.found = true;
            let role_ok = role_matches(&flat, row, director);
            result.contradicted |= contradicts(&flat, row, director);
            let institution = !school.is_empty()
                && (contains(&flat, &school) || contains(&heading, &school))
                && jurisdiction_matches(&flat, &heading, &row.state);
            if !institution || !role_ok { return; }
            let name_field = if director { "ad_name" } else { "coach_name" };
            result.fields.push(evidence(name_field, person, person, span, &context));
            if !email.is_empty() && contains(&flat, &normalize(email)) {
                let email_field = if director { "ad_email" } else { "public_professional_email" };
                result.fields.push(evidence(email_field, email, person, span, &context));
            }
        });
    });
    Ok(result)
}

fn spans(text: &str) -> anyhow::Result<(Vec<String>, String)> {
    if !text.contains('<') {
        return Ok((text.lines().filter(|line| !line.trim().is_empty()).map(str::to_string).collect(), String::new()));
    }
    let document = Html::parse_document(text);
    let records = Selector::parse("tr, li, article, [class~='staff-card'], [class~='coach-card'], [class~='staff-member']")
        .map_err(|error| anyhow::anyhow!("staff record selector: {error}"))?;
    let heading = Selector::parse("title, h1")
        .map_err(|error| anyhow::anyhow!("staff heading selector: {error}"))?;
    let headings = document.select(&heading).flat_map(|element| element.text()).collect::<Vec<_>>().join(" ");
    let records: Vec<String> = document.select(&records)
        .filter(|element| !element.select(&records).any(|child| child.id() != element.id()))
        .map(|element| element.text().collect::<Vec<_>>().join(" "))
        .collect();
    if !records.is_empty() { return Ok((records, normalize(&headings))); }
    let paragraphs = Selector::parse("p")
        .map_err(|error| anyhow::anyhow!("staff paragraph selector: {error}"))?;
    Ok((document.select(&paragraphs).map(|element| element.text().collect::<Vec<_>>().join(" ")).collect(), normalize(&headings)))
}

fn contains(text: &str, value: &str) -> bool {
    !value.is_empty() && text.match_indices(value).any(|(start, _)| {
        let end = start.saturating_add(value.len());
        let token = |ch: char| ch.is_alphanumeric() || matches!(ch, '@' | '_' | '.' | '-');
        !text.get(..start).and_then(|prefix| prefix.chars().next_back()).is_some_and(token)
            && !text.get(end..).and_then(|suffix| suffix.chars().next()).is_some_and(token)
    })
}

fn role_matches(span: &str, row: &FragmentRow, director: bool) -> bool {
    if director { return contains(span, "athletic director"); }
    let role = normalize(&row.role);
    contains(span, "coach") && program_matches(span, row)
        && ["head", "assistant", "boys", "girls"].into_iter()
            .all(|part| !contains(&role, part) || contains(span, part))
        && !(contains(&role, "head") && !contains(&role, "assistant") && contains(span, "assistant"))
}

fn program_matches(span: &str, row: &FragmentRow) -> bool {
    let sport = normalize(&row.sport);
    if sport.contains("cross") || contains(&sport, "xc") {
        return contains(span, "cross country") || contains(span, "cross-country") || contains(span, "xc");
    }
    sport.contains("track") && contains(span, "track")
}

fn contradicts(span: &str, row: &FragmentRow, director: bool) -> bool {
    if director { return !contains(span, "athletic director") && contains(span, "coach"); }
    (!program_matches(span, row)
        && ["basketball", "football", "soccer", "volleyball", "baseball", "swimming"]
            .into_iter().any(|sport| contains(span, sport)))
        || (contains(span, "athletic director") && !contains(span, "coach"))
        || (contains(&normalize(&row.role), "head") && contains(span, "assistant"))
}

fn jurisdiction_matches(span: &str, heading: &str, state: &str) -> bool {
    census_domain::UsJurisdiction::from_code(state).is_some_and(|jurisdiction| {
        let code = normalize(jurisdiction.code());
        let name = normalize(jurisdiction.name());
        [span, heading].into_iter().any(|text| contains(text, &code) || contains(text, &name))
    })
}

fn evidence(field: &str, value: &str, person: &str, span: &str, context: &ClaimContext<'_>) -> ClaimEvidence {
    let row = context.row;
    ClaimEvidence {
        field: field.to_string(), value: value.to_string(), person: person.to_string(),
        role: if field.starts_with("ad_") { "Athletic Director".to_string() } else { row.role.clone() },
        sport: row.sport.clone(), school: row.school.clone(), state: row.state.clone(),
        source_url: context.url.to_string(), retrieved_at: context.retrieved_at.to_string(),
        claimed_observed_on: row.last_observed.clone(), source_sha256: context.digest.to_string(), span: span.to_string(),
    }
}
