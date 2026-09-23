//! Row validation: judge whether a row is importable, plus helper checks.

use super::{email_re, phone_re, url_re, vacant_re, Row, PERSONAL_MAIL};

/// Judge a row for importability. Returns `None` when valid, `Some(reason)` when rejected.
pub(super) fn judge(row: &Row, state: &str) -> Option<String> {
    if let Some(r) = check_state(row, state) {
        return Some(r);
    }
    if let Some(r) = check_school(row) {
        return Some(r);
    }
    if let Some(r) = check_url(row) {
        return Some(r);
    }
    if let Some(r) = check_role(row) {
        return Some(r);
    }
    if let Some(r) = check_name_fields(row) {
        return Some(r);
    }
    if let Some(r) = check_contact_published(row) {
        return Some(r);
    }
    if let Some(r) = check_phone_fields(row) {
        return Some(r);
    }
    check_email_fields(row)
}

fn check_state(row: &Row, state: &str) -> Option<String> {
    if row.state.to_uppercase() != state.to_uppercase() {
        Some(format!(
            "state {:?} does not match fragment {}",
            row.state, state
        ))
    } else {
        None
    }
}

fn check_school(row: &Row) -> Option<String> {
    if row.school.trim().is_empty() {
        Some("missing school".to_string())
    } else {
        None
    }
}

fn check_url(row: &Row) -> Option<String> {
    if url_re().is_some_and(|re| !re.is_match(row.source_url.trim())) {
        Some(format!("source_url is not a URL: {:?}", row.source_url))
    } else {
        None
    }
}

fn check_role(row: &Row) -> Option<String> {
    let sport = row.sport.trim();
    let role = row.role.trim();
    let role_lower = role.to_lowercase();
    let is_director = role_lower.contains("director");
    let is_coach = role_lower.contains("coach");

    if is_director && is_coach {
        return Some(format!(
            "role names both a coach and a director: {:?}",
            role
        ));
    }

    if is_director {
        if !sport.is_empty() {
            return Some("director row must leave sport empty".to_string());
        }
    } else if is_coach {
        if !resolves_sport(&format!("{sport} {role}")) {
            return Some(format!(
                "no sport resolvable from sport={:?} role={:?}",
                sport, role
            ));
        }
        if row.coach_name.trim().is_empty() {
            return Some("coach row has no coach name".to_string());
        }
    } else {
        return Some(format!(
            "role is neither a coach nor a director label: {:?}",
            role
        ));
    }

    None
}

fn check_name_fields(row: &Row) -> Option<String> {
    if let Some(reason) = check_placeholder(&row.coach_name) {
        return Some(format!("placeholder name in coach_name: {:?}", reason));
    }
    if let Some(reason) = check_placeholder(&row.ad_name) {
        return Some(format!("placeholder name in ad_name: {:?}", reason));
    }
    None
}

fn check_contact_published(row: &Row) -> Option<String> {
    if row.coach_name.trim().is_empty()
        && row.ad_name.trim().is_empty()
        && row.public_professional_email.trim().is_empty()
        && row.ad_email.trim().is_empty()
    {
        return Some("no contact published".to_string());
    }
    None
}

fn check_phone_fields(row: &Row) -> Option<String> {
    for (field_name, value) in [
        ("coach_name", &row.coach_name),
        ("ad_name", &row.ad_name),
        ("school", &row.school),
        ("city", &row.city),
    ] {
        if phone_re().is_some_and(|re| re.is_match(value.trim())) {
            return Some(format!(
                "phone-like value in {field_name}: {:?}",
                value.trim()
            ));
        }
    }
    None
}

fn check_email_fields(row: &Row) -> Option<String> {
    for (field_name, value) in [
        ("public_professional_email", &row.public_professional_email),
        ("ad_email", &row.ad_email),
    ] {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        if email_re().is_some_and(|re| !re.is_match(&trimmed)) {
            return Some(format!("not an email in {field_name}: {:?}", trimmed));
        }
        let domain = trimmed.rsplit_once('@').map(|(_, d)| d.to_lowercase());
        if let Some(domain) = domain {
            if PERSONAL_MAIL.contains(&domain.as_str()) {
                return Some(format!(
                    "personal mail domain in {field_name}: {:?}",
                    trimmed
                ));
            }
        }
    }
    None
}

/// Check whether a name field contains a placeholder / vacant value.
pub(super) fn check_placeholder(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    if vacant_re().is_some_and(|re| re.is_match(trimmed)) {
        Some(trimmed.to_string())
    } else {
        None
    }
}

/// Mirror the Rust parser: a label resolves when it names a sport by substring.
pub(super) fn resolves_sport(text: &str) -> bool {
    let lowered = text.to_lowercase();
    lowered.contains("track")
        || lowered.contains("cross country")
        || lowered.contains("cross-country")
        || lowered.contains("indoor")
}
