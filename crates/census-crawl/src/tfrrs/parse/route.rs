use super::html::collapse_whitespace;
use super::season::{Season, YearToken};
use census_domain::model::{Gender, Sport};
use census_domain::UsJurisdiction;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListPath {
    pub id: String,
    pub slug: String,
    pub season: Option<Season>,
}

pub fn parse_team_path(path: &str) -> Option<TeamPath> {
    let mut segments = tail_after(route_path(path), "teams")?
        .split('/')
        .filter(|segment| !segment.is_empty());
    let route = segments.next()?.to_string();
    let file = segments.next()?;
    let stem = file.strip_suffix(".html").map_or(file, |value| value);
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamPath {
    pub route: String,
    pub slug: String,
    pub gender: Option<Gender>,
}

pub(super) fn athlete_id(href: &str) -> Option<u64> {
    segment_after(href, "athletes").and_then(|token| token.parse::<u64>().ok())
}

pub(super) fn meet_id(href: &str) -> Option<u64> {
    segment_after(href, "results").and_then(|token| token.parse::<u64>().ok())
}

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
        .map_or(file, |value| value)
        .trim_start_matches('_');
    let name = collapse_whitespace(&stem.replace('_', " "));
    (!name.is_empty()).then_some(name)
}

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

fn segment_after<'a>(href: &'a str, after: &str) -> Option<&'a str> {
    let mut segments = href.split('/').filter(|segment| !segment.is_empty());
    while let Some(segment) = segments.next() {
        if segment == after {
            return segments.next();
        }
    }
    None
}

fn route_path(path: &str) -> &str {
    path.split(['?', '#']).next().map_or(path, |value| value)
}

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

fn query_value<'a>(url: &'a str, name: &str) -> Option<&'a str> {
    let query = url.split_once('?')?.1;
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
}

pub fn list_filter(url: &str) -> Option<YearToken> {
    YearToken::parse(query_value(url, "year")?)
}

pub fn jurisdiction_of_url(url: &str) -> Option<UsJurisdiction> {
    let host = url.split_once("//").map_or(url, |(_, rest)| rest);
    let label = host
        .split(['/', '?', '#'])
        .next()?
        .strip_suffix(".tfrrs.org")?;
    UsJurisdiction::parse(&label.replace('_', " "))
}
