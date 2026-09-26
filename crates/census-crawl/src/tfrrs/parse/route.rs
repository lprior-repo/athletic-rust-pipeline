//! The URL shapes the host publishes, read from either spelling they appear in.
//!
//! A page prints its own navigation as a relative path and its result links as absolute URLs
//! (`https://indiana.tfrrs.org/teams/tf/…`, every row of the captured Indiana list), so every
//! reader here is anchored on the route's own marker segment rather than on the start of the
//! string.

use super::html::collapse_whitespace;
use super::season::{Season, YearToken};
use census_domain::model::{Gender, Sport};
use census_domain::UsJurisdiction;

/// Read the season a list path states (`/lists/5489/HSR_All_School_Performance_List/2026/i`).
///
/// The `i`/`o` season segment is the only season signal on a list page: the page carries no season
/// control of its own. `i` is verified by the captured Indiana list; `o` is its outdoor sibling on
/// the same route shape.
pub fn parse_list_path(path: &str) -> Option<ListPath> {
    let mut segments = tail_after(route_path(path), "lists")?
        .split('/')
        .filter(|segment| !segment.is_empty());
    let id = segments.next()?.to_string();
    let slug = segments.next()?.to_string();
    let year = segments.next().and_then(|token| token.parse::<i16>().ok());
    let sport = segments.next().and_then(|token| match token {
        "i" => Some(Sport::IndoorTrack),
        "o" => Some(Sport::OutdoorTrack),
        _ => None,
    });
    let season = match (year, sport) {
        (Some(year), Some(sport)) => Some(Season {
            year,
            sport: Some(sport),
            school_year_label: false,
        }),
        _ => None,
    };
    Some(ListPath { id, slug, season })
}

/// A performance-list route: the list id, its slug, and the season the path states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListPath {
    pub id: String,
    pub slug: String,
    pub season: Option<Season>,
}

/// Read the sport, slug and gender side a team path names (`/teams/tf/Lawrence_Central_m.html`).
pub fn parse_team_path(path: &str) -> Option<TeamPath> {
    let mut segments = tail_after(route_path(path), "teams")?
        .split('/')
        .filter(|segment| !segment.is_empty());
    let route = segments.next()?.to_string();
    let file = segments.next()?;
    let stem = file.strip_suffix(".html").unwrap_or(file);
    let (slug, gender) = match stem.rsplit_once('_') {
        Some((slug, "m")) => (slug.to_string(), Some(Gender::Boys)),
        Some((slug, "f")) => (slug.to_string(), Some(Gender::Girls)),
        _ => (stem.to_string(), None),
    };
    if slug.is_empty() {
        return None;
    }
    Some(TeamPath {
        route,
        slug,
        gender,
    })
}

/// A team route: the `tf`/`xc` segment, the team slug, and the gender side the file stem names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamPath {
    pub route: String,
    pub slug: String,
    pub gender: Option<Gender>,
}

/// The numeric athlete id in a route (`/athletes/8429704/Lawrence_Central/_Evan_Williams`).
pub(super) fn athlete_id(href: &str) -> Option<u64> {
    segment_after(href, "athletes").and_then(|token| token.parse::<u64>().ok())
}

/// The numeric meet id in a route (`/results/94159/Hoosier_State_Relays_Finals_Large_Schools`).
pub(super) fn meet_id(href: &str) -> Option<u64> {
    segment_after(href, "results").and_then(|token| token.parse::<u64>().ok())
}

/// The full name an athlete route slug states, or `None` when the route carries no name segment
/// (`/athletes/8429704.html`).
pub(super) fn href_name(href: &str) -> Option<String> {
    let mut segments = href.split('/').filter(|segment| !segment.is_empty());
    for segment in segments.by_ref() {
        if segment == "athletes" {
            break;
        }
    }
    let _id = segments.next()?;
    let _school = segments.next()?;
    let file = segments.next()?;
    let stem = file
        .strip_suffix(".html")
        .unwrap_or(file)
        .trim_start_matches('_');
    let name = collapse_whitespace(&stem.replace('_', " "));
    (!name.is_empty()).then_some(name)
}

/// The school an athlete route slug states (`/athletes/9264598/Pembroke/Orion_Baldoumas.html`).
pub(super) fn href_school(href: &str) -> Option<String> {
    let mut segments = href.split('/').filter(|segment| !segment.is_empty());
    for segment in segments.by_ref() {
        if segment == "athletes" {
            break;
        }
    }
    let _id = segments.next()?;
    let school = segments.next()?;
    let name = collapse_whitespace(&school.replace('_', " "));
    (!name.is_empty()).then_some(name)
}

/// The segment that follows `after` in a route.
fn segment_after<'a>(href: &'a str, after: &str) -> Option<&'a str> {
    let mut segments = href.split('/').filter(|segment| !segment.is_empty());
    while let Some(segment) = segments.next() {
        if segment == after {
            return segments.next();
        }
    }
    None
}

/// The path half of a route, with any query string removed (`…/2026/i?year=SR` → `…/2026/i`).
fn route_path(path: &str) -> &str {
    path.split(['?', '#']).next().unwrap_or(path)
}

/// Everything after a route's marker segment (`lists`, `teams`, `athletes`).
///
/// The marker is matched as a whole segment, never as a prefix: a school named `teams_2` is not the
/// marker. Empty segments fall out of the match on their own, so `//teams//x` reads the same as
/// `/teams/x`.
fn tail_after<'a>(path: &'a str, marker: &str) -> Option<&'a str> {
    let mut rest = path;
    while let Some((segment, tail)) = rest.split_once('/') {
        if segment == marker {
            return Some(tail);
        }
        rest = tail;
    }
    None
}

/// The value of one query parameter of a URL (`?year=SR&gender=m`).
fn query_value<'a>(url: &'a str, name: &str) -> Option<&'a str> {
    let query = url.split_once('?')?.1;
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
}

/// The `?year=<TOKEN>` view a list URL was requested with, when it names one.
///
/// The page's own filter control offers exactly the tokens [`YearToken::parse`] reads (the
/// captured Indiana list carries it: `<option value="SR">SR</option>` … `<option value="6">6`
/// `</option>`), and a filtered view only re-ranks rows — the row's own `Year` cell stays
/// authoritative (`map::row::grade_for`). A `?year=` that states no such token (`?year=2024` on
/// a tournament route) is no filter rather than a guess.
pub fn list_filter(url: &str) -> Option<YearToken> {
    YearToken::parse(query_value(url, "year")?)
}

/// The state a TFRRS host serves (`nh.tfrrs.org` → New Hampshire, `indiana.tfrrs.org` →
/// Indiana).
///
/// TFRRS publishes one instance per state, so a page's host is the only jurisdiction it states,
/// and school identity keys on it: two `Central` high schools in two states must stay two
/// schools. Both spellings the site uses read: the USPS code the NH instance publishes (`nh`)
/// and the state name the Indiana instance does (`indiana`, plus the `in.tfrrs.org` host its own
/// pages link).
///
/// `None` is a refusal, not a default: a page on a host this reader cannot place (the national
/// `www`, the sport instances) is skipped rather than minted into a state.
pub fn jurisdiction_of_url(url: &str) -> Option<UsJurisdiction> {
    let host = url.split_once("//").map_or(url, |(_, rest)| rest);
    let label = host
        .split(['/', '?', '#'])
        .next()?
        .strip_suffix(".tfrrs.org")?;
    UsJurisdiction::parse(&label.replace('_', " "))
}
