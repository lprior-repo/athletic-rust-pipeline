use super::super::HOST;
use super::bound_fetch_options;
use crate::{AdapterContext, CrawlError, CrawlResult};

const VERIFIED: &[(&str, &str, &str)] = &[
    ("IA", "Ankeny Christian Academy", "ankenychristian"),
    ("IA", "Beckman Catholic, Dyersville", "beckman"),
    ("IA", "Bishop Garrigan, Algona", "garrigan"),
    ("IA", "Bishop Heelan Catholic, Sioux City", "heelan"),
    (
        "IA",
        "Columbus Community, Columbus Junction",
        "columbusjunction",
    ),
    ("IA", "Coram Deo Academy", "coramdeoia"),
    ("IA", "Council Bluffs, Abraham Lincoln", "cblincoln"),
    ("IA", "Council Bluffs, Thomas Jefferson", "cbjefferson"),
    ("IA", "Dowling Catholic, West Des Moines", "dowling"),
    ("IA", "Empigo Academy", "empigo"),
    ("IA", "Gehlen Catholic, LeMars", "gehlen"),
    ("IA", "Interstate 35, Truro", "i35"),
    ("IA", "Iowa City, City High", "iccityhigh"),
    ("IA", "Kuemper Catholic, Carroll", "kuemper"),
    ("IA", "Manson Northwest Webster", "manson"),
    ("IA", "Marquette Catholic, Bellevue", "marquette"),
    ("IA", "Newman Catholic, Mason City", "newman"),
    ("IA", "Prairie, Cedar Rapids", "crprairie"),
    ("IA", "Prince of Peace Catholic, Clinton", "princeofpeace"),
    ("IA", "Sioux City, East", "sceast"),
    ("IA", "Sioux City, North", "scnorth"),
    ("IA", "Sioux City, West", "scwest"),
    ("IA", "South O'Brien, Paullina", "southobrien"),
    ("IA", "St. Mary's, Remsen", "stmaryremsen"),
    ("IA", "St. Mary's, Storm Lake", "stmarystormlake"),
    ("IA", "Notre Dame, Burlington", "burlingtonnotredame"),
    ("IA", "Waukee Northwest", "northwest"),
    ("IA", "West Central Valley, Stuart", "wcvalley"),
];

pub(in crate::bound) struct Index {
    schools: Vec<(String, String)>,
}

impl Index {
    pub(in crate::bound) fn new() -> Self {
        Self {
            schools: Vec::new(),
        }
    }

    pub(in crate::bound) fn add(&mut self, slug: String, name: String) {
        self.schools.push((slug, name));
    }

    pub(in crate::bound) fn len(&self) -> usize {
        self.schools.len()
    }

    pub(in crate::bound) fn has_slug(&self, slug: &str) -> bool {
        self.schools.iter().any(|(s, _)| s == slug)
    }

    pub(in crate::bound) fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.schools.iter().map(|(s, n)| (s.as_str(), n.as_str()))
    }
}

pub(in crate::bound) async fn fetch_school_index(
    ctx: &AdapterContext<'_>,
    state_code: &str,
) -> CrawlResult<Index> {
    let url = format!("{HOST}/{state_code}/schools");
    let fetch_opts = bound_fetch_options(ctx);

    let outcome = ctx.fetcher.get(&url, &fetch_opts).await?;
    if outcome.status != 200 {
        return Err(CrawlError::Schema {
            url: url.clone(),
            detail: format!("status {}", outcome.status),
        });
    }

    let html = String::from_utf8_lossy(&outcome.body);
    parse_school_index(&html, state_code)
}

pub(in crate::bound) fn parse_school_index(html: &str, state_code: &str) -> CrawlResult<Index> {
    let mut index = Index::new();
    let needle = format!(r#"href="/{state_code}/schools/"#);

    for line in html.lines() {
        let Some((_, rest)) = line.split_once(needle.as_str()) else {
            continue;
        };
        let Some((slug, rest)) = rest.split_once("\">") else {
            continue;
        };
        let label = match rest.split_once("</a>") {
            Some((label, _)) => label,
            None => rest,
        };
        let label = label.trim().to_string();
        let slug = slug.to_string();
        if !label.is_empty() && !index.has_slug(&slug) {
            index.add(slug, label);
        }
    }

    Ok(index)
}

pub(in crate::bound) struct SlugResolution {
    pub(in crate::bound) matched: Vec<(String, String)>,
    pub(in crate::bound) ambiguous: Vec<String>,
    pub(in crate::bound) unmatched: Vec<String>,
}

enum SlugMatch {
    Matched(String),
    Ambiguous,
    Unmatched,
}

pub(in crate::bound) fn resolve_slugs(schools: Vec<String>, index: &Index) -> SlugResolution {
    let mut resolution = SlugResolution {
        matched: Vec::new(),
        ambiguous: Vec::new(),
        unmatched: Vec::new(),
    };

    for name in schools {
        match resolve_one(&name, index) {
            SlugMatch::Matched(slug) => resolution.matched.push((name, slug)),
            SlugMatch::Ambiguous => resolution.ambiguous.push(name),
            SlugMatch::Unmatched => resolution.unmatched.push(name),
        }
    }

    resolution
}

fn resolve_one(name: &str, index: &Index) -> SlugMatch {
    let short = name.split(',').next().map_or(name, str::trim);

    let slug_short = slugify(short);
    let slug_full = slugify(name);
    if index.has_slug(&slug_short) {
        return SlugMatch::Matched(slug_short);
    }
    if index.has_slug(&slug_full) {
        return SlugMatch::Matched(slug_full);
    }
    if let Some(slug) = verified_slug(name) {
        if index.has_slug(&slug) {
            return SlugMatch::Matched(slug);
        }
    }

    let token_picks = filter_secondary(&token_candidates(index, name, short), index);
    if !token_picks.is_empty() {
        return single(token_picks);
    }

    if slug_short.len() >= 5 {
        let prefix_picks = filter_secondary(&prefix_candidates(index, &slug_short), index);
        if !prefix_picks.is_empty() {
            return single(prefix_picks);
        }
    }

    SlugMatch::Unmatched
}

fn single(candidates: Vec<&str>) -> SlugMatch {
    match candidates.as_slice() {
        [only] => SlugMatch::Matched((*only).to_string()),
        _ => SlugMatch::Ambiguous,
    }
}

fn token_candidates<'a>(index: &'a Index, name: &str, short: &str) -> Vec<&'a str> {
    let want_tokens = name_tokens(name);
    let head = short
        .split_whitespace()
        .next()
        .map_or_else(String::new, |h| h.to_uppercase());

    index
        .iter()
        .filter(|(_slug, label)| {
            let label_upper = label.to_uppercase();
            let label_tokens = name_tokens(label);
            want_tokens.iter().all(|t| label_tokens.contains(t)) && label_upper.starts_with(&head)
        })
        .map(|(slug, _)| slug)
        .collect()
}

fn prefix_candidates<'a>(index: &'a Index, slug_short: &str) -> Vec<&'a str> {
    index
        .iter()
        .filter(|(_slug, label)| slugify(label).starts_with(slug_short))
        .map(|(slug, _)| slug)
        .collect()
}

pub(in crate::bound) fn verified_slug(name: &str) -> Option<String> {
    let straight = name.replace(['\u{2019}', '\u{2018}'], "'");
    for (state, school, slug) in VERIFIED {
        if *state == "IA" && *school == straight {
            return Some(slug.to_string());
        }
    }
    None
}

pub(in crate::bound) fn slugify(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

pub(in crate::bound) fn name_tokens(value: &str) -> Vec<String> {
    value
        .to_uppercase()
        .split_whitespace()
        .filter(|token| {
            !token.is_empty()
                && *token != "HIGH"
                && *token != "SCHOOL"
                && *token != "COMMUNITY"
                && *token != "SR"
                && *token != "SENIOR"
                && *token != "HOME"
                && *token != "THER"
        })
        .map(|t| t.to_string())
        .collect()
}

pub(in crate::bound) fn filter_secondary<'a>(
    candidates: &[&'a str],
    index: &Index,
) -> Vec<&'a str> {
    let mut varsity: Vec<&str> = Vec::new();
    for slug in candidates {
        let is_secondary = slug.ends_with("ms")
            || slug.contains("middleschool")
            || slug.contains("elementary")
            || {
                let label = index.iter().find(|(s, _)| s == slug).map_or("", |(_, l)| l);
                label.to_lowercase().contains("middle school")
                    || label.to_lowercase().contains("elementary")
                    || label.to_lowercase().contains("intermediate")
            };
        if is_secondary {
            continue;
        }
        varsity.push(slug);
    }
    varsity
}
