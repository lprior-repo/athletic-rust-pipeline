use super::patterns::{CONTEXT_MAX, CONTEXT_RADIUS, MAX_TABLES, MAX_TABLE_ROWS, TITLE_WORDS};
use super::text::{decode_entities, truncate};
use super::Rules;
use census_domain::model::{CoachRole, Gender, Sport};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoachHit {
    pub name: String,
    pub sport: Sport,
    pub context: String,
    pub url: String,
    pub title: String,
    pub role: CoachRole,
    pub gender: Gender,
    pub email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdHit {
    pub name: String,
    pub context: String,
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Signals {
    pub emails: Vec<String>,
    pub coach_hits: Vec<CoachHit>,
    pub ad_hits: Vec<AdHit>,
    pub pages: Vec<String>,
}

impl Signals {
    pub fn finish(&mut self) {
        self.emails.sort();
        self.emails.dedup();
    }

    pub fn has_signals(&self) -> bool {
        !self.emails.is_empty() || !self.coach_hits.is_empty() || !self.ad_hits.is_empty()
    }
}

pub fn analyse(rules: &Rules, url: &str, html: &str, signals: &mut Signals) {
    let text = rules.visible_text(html);
    let title = rules.title_of(html);
    let mut emails = harvest_emails(rules, html, &text);
    let flat = rules.flat_text(&text);
    collect_coaches(rules, url, &title, &flat, signals);
    collect_directors(rules, url, &title, &flat, signals);
    signals.pages.push(url.to_string());
    analyse_tables(rules, url, html, &title, signals);
    let mut combined = std::mem::take(&mut signals.emails);
    combined.append(&mut emails);
    signals.emails = rules.sanitize_emails(combined.iter().map(String::as_str));
}

fn harvest_emails(rules: &Rules, html: &str, text: &str) -> Vec<String> {
    let mut emails: Vec<String> = rules
        .mailto
        .captures_iter(html)
        .filter_map(|captures| captures.get(1))
        .map(|matched| matched.as_str().to_string())
        .collect();
    emails.extend(
        rules
            .bare_mail
            .find_iter(text)
            .map(|matched| matched.as_str().to_string()),
    );
    emails
}

fn collect_coaches(rules: &Rules, url: &str, title: &str, flat: &str, signals: &mut Signals) {
    for pattern in &rules.coach {
        for captures in pattern.captures_iter(flat) {
            let Some(name) = captures
                .get(1)
                .and_then(|m| clean_person(rules, m.as_str()))
            else {
                continue;
            };
            let window = window_of(rules, flat, captures.get(0));
            if rules.junk.is_match(&window) {
                continue;
            }
            signals.coach_hits.push(CoachHit {
                name,
                sport: sport_of(rules, &window),
                context: truncate(&window, CONTEXT_MAX).to_string(),
                url: url.to_string(),
                title: title.to_string(),
                role: role_of(rules, &window),
                gender: gender_of(rules, &window),
                email: None,
            });
        }
    }
}

fn collect_directors(rules: &Rules, url: &str, title: &str, flat: &str, signals: &mut Signals) {
    for pattern in &rules.ad {
        for captures in pattern.captures_iter(flat) {
            let Some(name) = captures
                .get(1)
                .and_then(|m| clean_person(rules, m.as_str()))
            else {
                continue;
            };
            let window = window_of(rules, flat, captures.get(0));
            if rules.junk.is_match(&window) {
                continue;
            }
            signals.ad_hits.push(AdHit {
                name,
                context: truncate(&window, CONTEXT_MAX).to_string(),
                url: url.to_string(),
                title: title.to_string(),
            });
        }
    }
}

fn analyse_tables(rules: &Rules, url: &str, html: &str, title: &str, signals: &mut Signals) {
    for (table_index, table) in rules.table.captures_iter(html).enumerate() {
        if table_index >= MAX_TABLES {
            break;
        }
        let body = table.get(1).map_or("", |matched| matched.as_str());
        let rows: Vec<&str> = rules
            .row
            .captures_iter(body)
            .filter_map(|captures| captures.get(1))
            .map(|matched| matched.as_str())
            .collect();
        if rows.len() < 2 {
            continue;
        }
        if !rules.table_header.is_match(&header_of(rules, rows.first())) {
            continue;
        }
        for row in rows.iter().skip(1).take(MAX_TABLE_ROWS) {
            if let Some(hit) = table_hit(rules, row, url, title) {
                signals.coach_hits.push(hit);
            }
        }
    }
}

fn header_of(rules: &Rules, row: Option<&&str>) -> String {
    rules
        .collapse_spaces(&decode_entities(
            &rules.tag.replace_all(row.map_or("", |value| *value), " "),
        ))
        .to_ascii_lowercase()
}

fn table_hit(rules: &Rules, row: &str, url: &str, title: &str) -> Option<CoachHit> {
    let cells: Vec<String> = rules
        .cell
        .captures_iter(row)
        .filter_map(|captures| captures.get(1))
        .map(|matched| {
            rules.collapse_spaces(&decode_entities(
                &rules.tag.replace_all(matched.as_str(), " "),
            ))
        })
        .filter(|cell| !cell.is_empty())
        .collect();
    if cells.len() < 2 {
        return None;
    }
    let joined = cells.join(" | ");
    if !rules.sport_hint.is_match(&joined) {
        return None;
    }
    let name = cells.iter().find_map(|cell| clean_person(rules, cell))?;
    let email = cells
        .iter()
        .find_map(|cell| rules.bare_mail.find(cell))
        .map(|matched| matched.as_str().to_string());
    Some(CoachHit {
        name,
        sport: sport_of(rules, &joined),
        context: truncate(&joined, CONTEXT_MAX).to_string(),
        url: url.to_string(),
        title: title.to_string(),
        role: role_of(rules, &joined),
        gender: gender_of(rules, &joined),
        email,
    })
}

fn window_of(rules: &Rules, flat: &str, matched: Option<regex::Match<'_>>) -> String {
    let Some(matched) = matched else {
        return String::new();
    };
    let start = matched.start().saturating_sub(CONTEXT_RADIUS);
    let end = matched.end().saturating_add(CONTEXT_RADIUS).min(flat.len());
    let window = flat.get(start..end).map_or("", |slice| slice);
    rules.collapse_spaces(window)
}

fn sport_of(rules: &Rules, text: &str) -> Sport {
    if rules.cross_country.is_match(text) {
        Sport::CrossCountry
    } else {
        Sport::OutdoorTrack
    }
}

fn role_of(rules: &Rules, text: &str) -> CoachRole {
    if rules.assistant.is_match(text) {
        CoachRole::AssistantCoach
    } else {
        CoachRole::HeadCoach
    }
}

fn gender_of(rules: &Rules, text: &str) -> Gender {
    if rules.girls.is_match(text) {
        Gender::Girls
    } else if rules.boys.is_match(text) {
        Gender::Boys
    } else {
        Gender::Unknown
    }
}

pub fn clean_person(rules: &Rules, name: &str) -> Option<String> {
    let mut parts: Vec<&str> = name.split_whitespace().collect();
    while parts.first().is_some_and(|part| trim_word(part)) {
        parts.remove(0);
    }
    while parts.last().is_some_and(|part| trim_word(part)) {
        parts.pop();
    }
    let candidate = parts.join(" ");
    if valid_person(rules, &candidate) {
        Some(candidate)
    } else {
        None
    }
}

fn trim_word(part: &str) -> bool {
    let token = part.trim_matches('.').to_ascii_lowercase();
    TITLE_WORDS.contains(&token.as_str()) || token.len() == 1
}

fn valid_person(rules: &Rules, name: &str) -> bool {
    let parts: Vec<&str> = name.split_whitespace().collect();
    if parts.len() < 2 || parts.len() > 4 {
        return false;
    }
    for part in parts {
        let token = part.trim_matches('.').to_ascii_lowercase();
        if TITLE_WORDS.contains(&token.as_str()) {
            return false;
        }
        if rules.token.find(part).map(|m| m.as_str()) != Some(part) {
            return false;
        }
    }
    true
}
