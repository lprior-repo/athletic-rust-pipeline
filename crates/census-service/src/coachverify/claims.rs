use super::normalize;
use super::spans::spans;
use census_domain::model::{ContactClaimEvidence, ContactProofField, RawContactRow};

pub(super) const MAX_SPAN: usize = 4096;

pub(super) struct PageClaims {
    pub fields: Vec<ContactClaimEvidence>,
    pub found: bool,
    pub contradicted: bool,
}

struct ClaimContext<'a> {
    row: &'a RawContactRow,
    url: &'a str,
    fetched_at: &'a str,
    source_sha256: &'a str,
}

pub(super) fn inspect(
    text: &str,
    row: &RawContactRow,
    url: &str,
    source_sha256: &str,
    fetched_at: &str,
) -> PageClaims {
    let (spans, heading) = spans(text);
    let context = ClaimContext {
        row,
        url,
        fetched_at,
        source_sha256,
    };
    let mut result = PageClaims {
        fields: Vec::new(),
        found: false,
        contradicted: false,
    };
    for span in spans {
        if span.len() > MAX_SPAN {
            continue;
        }
        for director in [false, true] {
            collect(&span, &heading, director, &context, &mut result);
        }
    }
    result
}

fn collect(
    span: &str,
    heading: &str,
    director: bool,
    context: &ClaimContext<'_>,
    result: &mut PageClaims,
) {
    let row = context.row;
    let flat = normalize(span);
    let (person, email) = if director {
        (&row.ad_name, &row.ad_email)
    } else {
        (&row.coach_name, &row.public_professional_email)
    };
    if person.is_empty() || !contains(&flat, &normalize(person)) {
        return;
    }
    let row_ok = if director {
        contains(&flat, "athletic director")
    } else {
        role_matches(&flat, row) && institution_matches(&flat, heading, row)
    };
    if !row_ok {
        return;
    }
    result.contradicted |= !director && self_contradicts(&flat, row);
    let name = if director {
        ContactProofField::AdName
    } else {
        ContactProofField::CoachName
    };
    result
        .fields
        .push(evidence(name, person, person, span, context));
    if !email.is_empty() && contains(&flat, &normalize(email)) {
        let field = if director {
            ContactProofField::AdEmail
        } else {
            ContactProofField::PublicProfessionalEmail
        };
        result
            .fields
            .push(evidence(field, email, person, span, context));
    }
}

fn contains(text: &str, value: &str) -> bool {
    !value.is_empty()
        && text.match_indices(value).any(|(start, _)| {
            let end = start.saturating_add(value.len());
            let token = |ch: char| ch.is_alphanumeric() || matches!(ch, '@' | '_' | '.' | '-');
            !text
                .get(..start)
                .and_then(|prefix| prefix.chars().next_back())
                .is_some_and(token)
                && !text
                    .get(end..)
                    .and_then(|suffix| suffix.chars().next())
                    .is_some_and(token)
        })
}

fn role_matches(flat: &str, row: &RawContactRow) -> bool {
    let role = normalize(&row.role);
    contains(flat, "coach")
        && program_matches(flat, row)
        && ["head", "assistant", "boys", "girls"]
            .into_iter()
            .all(|part| !contains(&role, part) || contains(flat, part))
        && !(contains(&role, "head")
            && !contains(&role, "assistant")
            && contains(flat, "assistant"))
}

fn program_matches(flat: &str, row: &RawContactRow) -> bool {
    let sport = normalize(&row.sport);
    if sport.contains("cross") || contains(&sport, "xc") {
        return contains(flat, "cross country")
            || contains(flat, "cross-country")
            || contains(flat, "xc");
    }
    sport.contains("track") && contains(flat, "track")
}

fn self_contradicts(flat: &str, row: &RawContactRow) -> bool {
    let negation_marker =
        contains(flat, "former") || contains(flat, "not current") || contains(flat, "no longer");
    let role = normalize(&row.role);
    negation_marker && contains(flat, &role) && contains(flat, "coach")
}

fn institution_matches(flat: &str, heading: &str, row: &RawContactRow) -> bool {
    let school = normalize(&row.school);
    !school.is_empty()
        && (contains(flat, &school) || contains(heading, &school))
        && jurisdiction_matches(flat, heading, &row.state)
}

fn jurisdiction_matches(span: &str, heading: &str, state: &str) -> bool {
    census_domain::UsJurisdiction::from_code(state).is_some_and(|jurisdiction| {
        let code = normalize(jurisdiction.code());
        let name = normalize(jurisdiction.name());
        [span, heading]
            .into_iter()
            .any(|text| contains(text, &code) || contains(text, &name))
    })
}

fn evidence(
    field: ContactProofField,
    value: &str,
    person: &str,
    span: &str,
    context: &ClaimContext<'_>,
) -> ContactClaimEvidence {
    let row = context.row;
    let role = if field == ContactProofField::AdName {
        "Athletic Director".to_string()
    } else {
        row.role.clone()
    };
    ContactClaimEvidence {
        field,
        value: value.to_string(),
        person: person.to_string(),
        role,
        sport: row.sport.clone(),
        school: row.school.clone(),
        state: row.state.clone(),
        source_url: context.url.to_string(),
        claimed_observed_on: row.last_observed.clone(),
        source_sha256: context.source_sha256.to_string(),
        fetched_at: context.fetched_at.to_string(),
        span: span.to_string(),
    }
}
