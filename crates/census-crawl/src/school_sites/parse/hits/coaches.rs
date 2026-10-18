use super::{clean_person, gender_of, role_of, sport_of, truncate, CoachHit, Rules, Signals};
use crate::coach_directories::SchoolCoachBlock;
use crate::CrawlResult;

pub(super) fn collect(
    rules: &Rules,
    url: &str,
    title: &str,
    html: &str,
    signals: &mut Signals,
) -> CrawlResult<()> {
    let blocks = crate::coach_directories::school_coach_blocks(html.as_bytes())?;
    for block in &blocks {
        if rules.junk.is_match(&block.text)
            || block
                .heading
                .as_deref()
                .is_some_and(|heading| rules.junk.is_match(heading))
            || multiple_appointments(&block.text)
            || ambiguous_people(rules, &block.text)
        {
            continue;
        }
        if let Some(hit) = heading_coach(rules, url, title, block) {
            retain(signals, hit)?;
        }
        for pattern in &rules.coach {
            for captures in pattern.captures_iter(&block.text) {
                if let Some(name) = captures
                    .get(1)
                    .and_then(|m| clean_person(rules, m.as_str()))
                {
                    let Some(matched) = captures.get(0) else {
                        continue;
                    };
                    let mut candidate = hit(rules, url, title, name, matched.as_str());
                    candidate.role = matched_role(rules, &block.text, matched);
                    retain(signals, candidate)?;
                }
            }
        }
    }
    Ok(())
}

fn matched_role(
    rules: &Rules,
    text: &str,
    matched: regex::Match<'_>,
) -> census_domain::model::CoachRole {
    let previous = text
        .get(..matched.start())
        .and_then(|prefix| prefix.split_whitespace().next_back());
    if previous.is_some_and(|word| {
        let word = word.trim_matches(|ch: char| !ch.is_alphabetic());
        word.eq_ignore_ascii_case("assistant") || word.eq_ignore_ascii_case("asst")
    }) {
        census_domain::model::CoachRole::AssistantCoach
    } else {
        role_of(rules, matched.as_str())
    }
}

fn heading_coach(
    rules: &Rules,
    url: &str,
    title: &str,
    block: &SchoolCoachBlock,
) -> Option<CoachHit> {
    let heading = block.heading.as_deref()?;
    if !sport_heading(heading) {
        return None;
    }
    let (role, raw_name) = block.text.split_once(':')?;
    if ![
        "head coach",
        "assistant coach",
        "asst coach",
        "asst. coach",
        "coach",
    ]
    .iter()
    .any(|label| role.trim().eq_ignore_ascii_case(label))
    {
        return None;
    }
    let name = clean_person(
        rules,
        raw_name.split_once('(').map_or(raw_name, |(name, _)| name),
    )?;
    let context = format!("{heading} | {}", block.text);
    let mut candidate = hit(rules, url, title, name, &context);
    candidate.role = role_of(rules, role);
    Some(candidate)
}

fn sport_heading(heading: &str) -> bool {
    let words: Vec<_> = heading
        .split(|ch: char| !ch.is_alphabetic())
        .filter(|word| !word.is_empty())
        .collect();
    words.iter().any(|word| word.eq_ignore_ascii_case("track") || word.eq_ignore_ascii_case("xc"))
        || words.windows(2).any(|pair| {
            matches!(pair, [first, second] if first.eq_ignore_ascii_case("cross") && second.eq_ignore_ascii_case("country"))
        })
}

fn multiple_appointments(text: &str) -> bool {
    text.split(|ch: char| !ch.is_alphabetic())
        .filter(|word| word.eq_ignore_ascii_case("coach"))
        .take(2)
        .count()
        > 1
}

fn ambiguous_people(rules: &Rules, text: &str) -> bool {
    let mut first = None;
    for pattern in &rules.coach {
        for captures in pattern.captures_iter(text) {
            let Some(name) = captures
                .get(1)
                .and_then(|matched| clean_person(rules, matched.as_str()))
            else {
                continue;
            };
            if first.as_ref().is_some_and(|first| *first != name) {
                return true;
            }
            first = Some(name);
        }
    }
    false
}

fn hit(rules: &Rules, url: &str, title: &str, name: String, context: &str) -> CoachHit {
    CoachHit {
        name,
        sport: sport_of(rules, context),
        context: truncate(context, super::CONTEXT_MAX).to_owned(),
        url: url.to_owned(),
        title: title.to_owned(),
        role: role_of(rules, context),
        gender: gender_of(rules, context),
        email: None,
    }
}

fn retain(signals: &mut Signals, hit: CoachHit) -> CrawlResult<()> {
    if !signals.coach_hits.iter().any(|existing| {
        existing.name == hit.name
            && existing.sport == hit.sport
            && existing.role == hit.role
            && existing.gender == hit.gender
            && existing.url == hit.url
    }) {
        if signals.coach_hits.len() >= 16_384 {
            return Err(crate::CrawlError::Resource {
                resource: "school site retained coach rows",
                requested: 16_385,
                limit: 16_384,
            });
        }
        signals.coach_hits.push(hit);
    }
    Ok(())
}
