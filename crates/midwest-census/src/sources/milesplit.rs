//! MileSplit HTML adapter — the 12-state discovery layer.
//!
//! Only robots-permitted, server-rendered surfaces are used:
//!
//! * `/{teams}` — the per-state team index (one row per team: name, location, team id)
//! * `/{teams}/<id>-<slug>/roster` — the graded roster table
//! * `/{athletes}/<id>-<slug>` — public athlete profile (used only for confirmation, never required)
//!
//! `/api/`, `/rankings`, `/virtual-meets` and `/contact` are robots-disallowed and are never
//! requested; the roster HTML carries the same graduating-year evidence the JSON API would provide.

use crate::model::*;
use crate::net::{FetchOptions, Fetcher};
use anyhow::{bail, Context, Result};
use regex::Regex;
use std::sync::LazyLock;

/// One MileSplit state site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Site {
    pub state: &'static str,
    pub host: &'static str,
}

pub const SITES: [Site; 12] = [
    Site {
        state: "WI",
        host: "wi.milesplit.com",
    },
    Site {
        state: "MN",
        host: "mn.milesplit.com",
    },
    Site {
        state: "IA",
        host: "ia.milesplit.com",
    },
    Site {
        state: "IL",
        host: "il.milesplit.com",
    },
    Site {
        state: "MI",
        host: "mi.milesplit.com",
    },
    Site {
        state: "IN",
        host: "in.milesplit.com",
    },
    Site {
        state: "OH",
        host: "oh.milesplit.com",
    },
    Site {
        state: "MO",
        host: "mo.milesplit.com",
    },
    Site {
        state: "KS",
        host: "ks.milesplit.com",
    },
    Site {
        state: "NE",
        host: "ne.milesplit.com",
    },
    Site {
        state: "ND",
        host: "nd.milesplit.com",
    },
    Site {
        state: "SD",
        host: "sd.milesplit.com",
    },
];

impl Site {
    pub fn for_state(code: &str) -> Option<Site> {
        SITES
            .iter()
            .copied()
            .find(|site| site.state.eq_ignore_ascii_case(code))
    }

    pub fn teams_url(&self) -> String {
        format!("https://{}/teams", self.host)
    }

    pub fn source_id(&self) -> String {
        format!("milesplit_{}", self.state.to_ascii_lowercase())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamRef {
    pub id: String,
    pub slug: String,
    pub url: String,
    pub name: String,
    pub city_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterAthlete {
    pub roster_name: String,
    pub name: String,
    pub gender: Gender,
    pub grad_year: GradYear,
    pub athlete_id: String,
    pub profile_url: String,
    pub indoor: bool,
    pub outdoor: bool,
    pub xc: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roster {
    pub team: TeamRef,
    pub athletes: Vec<RosterAthlete>,
}

static TEAM_ROW_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
            r#"(?s)<tr>\s*<td>\s*<a href="(https?://[a-z]{2}\.milesplit\.com/teams/(\d+)-([^"]+))">\s*([^<]+?)\s*</a>\s*</td>\s*<td>\s*([^<]*?)\s*</td>"#,
        )
        .expect("valid team-row regex")
});

static ATHLETE_ROW_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<li class="athlete-row data-row">(.*?)</li>"#).expect("row regex")
});

static ATHLETE_LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<a href="(https?://[a-z]{2}\.milesplit\.com/athletes/(\d+)-[^"]*)">([^<]+)</a>"#)
        .expect("athlete link regex")
});

static GENDER_CELL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"column-gender[^>]*>([^<]*)<"#).expect("gender regex"));

static GRAD_CELL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"column-grad-year[^>]*>([^<]*)<"#).expect("grad regex"));

static SEASON_CELL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
            r#"(?s)<div class="data-point[^"]*"[^>]*data-season-id="(\d+)"[^>]*>\s*<svg[^>]*class="icon icon-(yes|no)""#,
        )
        .expect("season regex")
});

/// Parse the per-state team index page.
pub fn parse_team_index(html: &str) -> Result<Vec<TeamRef>> {
    let mut teams = Vec::new();
    for capture in TEAM_ROW_REGEX.captures_iter(html) {
        let url = capture.get(1).expect("group 1").as_str().to_string();
        let id = capture.get(2).expect("group 2").as_str().to_string();
        let slug = capture.get(3).expect("group 3").as_str().to_string();
        let name = capture.get(4).expect("group 4").as_str().trim().to_string();
        let city_state = capture.get(5).expect("group 5").as_str().trim().to_string();
        if name.is_empty() {
            continue;
        }
        teams.push(TeamRef {
            id,
            slug,
            url,
            name,
            city_state,
        });
    }
    if teams.is_empty() {
        bail!("team index contained no team rows (markup change or empty state page)");
    }
    Ok(teams)
}

/// Parse a graded roster page.
pub fn parse_roster(html: &str, team: TeamRef) -> Result<Roster> {
    let link = &ATHLETE_LINK_REGEX;
    let gender_cell = &GENDER_CELL_REGEX;
    let grad_cell = &GRAD_CELL_REGEX;
    let season_cell = &SEASON_CELL_REGEX;

    let mut athletes = Vec::new();
    for row in ATHLETE_ROW_REGEX.captures_iter(html) {
        let row_html = row.get(1).expect("row").as_str();
        let Some(athlete) = link.captures(row_html) else {
            continue;
        };
        let profile_url = athlete.get(1).expect("url").as_str().to_string();
        let athlete_id = athlete.get(2).expect("id").as_str().to_string();
        let roster_name = html_unescape(athlete.get(3).expect("name").as_str().trim());
        if roster_name.is_empty() {
            continue;
        }
        let gender = gender_cell
            .captures(row_html)
            .map(|capture| Gender::parse_milesplit(capture.get(1).expect("gender").as_str()))
            .unwrap_or(Gender::Unknown);
        let Some(grad_year) = grad_cell
            .captures(row_html)
            .and_then(|capture| {
                capture
                    .get(1)
                    .expect("grad")
                    .as_str()
                    .trim()
                    .parse::<i16>()
                    .ok()
            })
            .and_then(GradYear::new)
        else {
            continue;
        };
        let mut seasons = Vec::new();
        for capture in season_cell.captures_iter(row_html) {
            let season_id: u8 = capture
                .get(1)
                .expect("season id")
                .as_str()
                .parse()
                .unwrap_or(0);
            let active = capture.get(2).expect("icon").as_str() == "yes";
            seasons.push((season_id, active));
        }
        // Cells appear in header order: Indoor, Outdoor, XC. The `data-season-id` is informational
        // (1 = indoor, 2 = outdoor, 3 = XC); position is authoritative.
        let indoor = seasons.first().map(|(_, active)| *active).unwrap_or(false);
        let outdoor = seasons.get(1).map(|(_, active)| *active).unwrap_or(false);
        let xc = seasons.get(2).map(|(_, active)| *active).unwrap_or(false);

        athletes.push(RosterAthlete {
            name: flip_last_first(&roster_name),
            roster_name,
            gender,
            grad_year,
            athlete_id,
            profile_url,
            indoor,
            outdoor,
            xc,
        });
    }
    Ok(Roster { team, athletes })
}

fn html_unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}

/// Fetch and parse the team index for a state site.
pub async fn fetch_team_index(
    fetcher: &Fetcher,
    site: Site,
    options: &FetchOptions,
) -> Result<Vec<TeamRef>> {
    let outcome = fetcher.get(&site.teams_url(), options).await?;
    if outcome.status != 200 {
        bail!(
            "team index {} returned HTTP {}",
            outcome.url,
            outcome.status
        );
    }
    parse_team_index(&outcome.text())
}

/// Fetch and parse one roster.
pub async fn fetch_roster(
    fetcher: &Fetcher,
    team: &TeamRef,
    options: &FetchOptions,
) -> Result<Roster> {
    let url = format!("{}/roster", team.url);
    let outcome = fetcher
        .get(
            &url,
            &FetchOptions {
                allow_not_found: true,
                ..options.clone()
            },
        )
        .await?;
    if outcome.status == 404 {
        return Ok(Roster {
            team: team.clone(),
            athletes: Vec::new(),
        });
    }
    if outcome.status != 200 {
        bail!("roster {} returned HTTP {}", url, outcome.status);
    }
    parse_roster(&outcome.text(), team.clone())
}

impl RosterAthlete {
    pub fn sports(&self) -> Vec<Sport> {
        let mut sports = Vec::new();
        if self.indoor {
            sports.push(Sport::IndoorTrack);
        }
        if self.outdoor {
            sports.push(Sport::OutdoorTrack);
        }
        if self.xc {
            sports.push(Sport::CrossCountry);
        }
        sports
    }

    /// The school year this roster was observed in. Rosters are current-season documents; the
    /// caller passes the school year the collection belongs to.
    pub fn observed_grade(
        &self,
        school_year: SchoolYear,
        source: SourceRef,
    ) -> Option<ObservedGrade> {
        let grade_number = 13 - (self.grad_year.0 - school_year.start_year());
        Grade::new(u8::try_from(grade_number).ok()?).map(|grade| ObservedGrade {
            grade,
            school_year,
            source,
        })
    }
}

/// Convert a parsed roster into canonical entities.
pub fn roster_entities(
    roster: &Roster,
    state: &str,
    school_year: SchoolYear,
    observed_on: &str,
    site: &Site,
) -> (CanonicalSchool, Vec<CanonicalAthlete>, Vec<CanonicalTeam>) {
    let source = SourceRef::new(
        site.source_id(),
        Some(format!("{}/roster", roster.team.url)),
    );
    let owner = owner_name(&roster.team);
    let (mut school, school_id) = CanonicalSchool::new(state, &owner, normalize_name(&owner));
    school.city = city_of(&roster.team.city_state);
    school.source_identities.push(
        SourceIdentity::new(SourceNamespace::MilesplitSchool, roster.team.id.clone())
            .with_url(roster.team.url.clone()),
    );
    school
        .evidence
        .push(Evidence::parsed(source.clone(), observed_on.to_string()));

    let mut seen_sports: Vec<(Sport, Gender)> = Vec::new();
    let mut athletes = Vec::new();
    for entry in &roster.athletes {
        for sport in entry.sports() {
            let key = (sport, entry.gender);
            if !seen_sports.contains(&key) {
                seen_sports.push(key);
            }
        }
        let mut athlete = CanonicalAthlete::new(
            &school_id,
            entry.name.clone(),
            entry.grad_year,
            entry.gender,
        );
        athlete.known_names = vec![entry.name.clone(), entry.roster_name.clone()];
        athlete.sports = entry.sports();
        if let Some(observation) = entry.observed_grade(school_year, source.clone()) {
            athlete.observed_grades.push(observation);
        }
        athlete.public_profile_urls.push(entry.profile_url.clone());
        athlete.source_identities.push(
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, entry.athlete_id.clone())
                .with_url(entry.profile_url.clone()),
        );
        athlete.evidence.push(Evidence::parsed(
            SourceRef::new(site.source_id(), Some(entry.profile_url.clone())),
            observed_on.to_string(),
        ));
        athlete.identity_confidence = if entry.grad_year == GradYear::CO2027 {
            Confidence::HIGH
        } else {
            Confidence::MEDIUM
        };
        athletes.push(athlete);
    }

    let teams = seen_sports
        .into_iter()
        .map(|(sport, gender)| {
            let id = Id::<tag::Team>::mint(
                "team",
                &[
                    school_id.as_str(),
                    &format!("{sport:?}"),
                    &format!("{gender:?}"),
                    &school_year.start_year().to_string(),
                ],
            );
            CanonicalTeam {
                id,
                school: school_id.clone(),
                sport,
                gender,
                school_year,
                level: Some("high_school".to_string()),
                source_identities: vec![SourceIdentity::new(
                    SourceNamespace::MilesplitTeam,
                    roster.team.id.clone(),
                )
                .with_url(roster.team.url.clone())],
                evidence: vec![Evidence::parsed(source.clone(), observed_on.to_string())],
            }
        })
        .collect();

    (school, athletes, teams)
}

/// MileSplit team rows use the school name; strip a trailing gender marker if present.
fn owner_name(team: &TeamRef) -> String {
    let name = team.name.trim();
    for suffix in [" Boys", " Girls", " (B)", " (G)"] {
        if let Some(stripped) = name.strip_suffix(suffix) {
            return stripped.trim().to_string();
        }
    }
    name.to_string()
}

fn city_of(city_state: &str) -> Option<String> {
    let first = city_state.split(',').next()?.trim();
    if first.is_empty() {
        None
    } else {
        Some(title_case(first))
    }
}

fn title_case(value: &str) -> String {
    value
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str().to_lowercase()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// MileSplit's own site id (`wi`, `mn`, …) for a state code.
pub fn site_for_state(code: &str) -> Result<Site> {
    Site::for_state(code).with_context(|| format!("no MileSplit site registered for {code}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEAMS: &str = include_str!("../../tests/fixtures/milesplit/wi_teams_index.html");
    const ROSTER: &str = include_str!("../../tests/fixtures/milesplit/wi_roster_52649.html");

    #[test]
    fn parses_team_index_rows() {
        let teams = parse_team_index(TEAMS).unwrap();
        assert_eq!(teams.len(), 40);
        assert_eq!(teams[0].id, "52649");
        assert_eq!(teams[0].name, "Abbotsford");
        assert_eq!(teams[0].city_state, "ABBOTSFORD, WI, USA");
    }

    #[test]
    fn parses_roster_rows_with_grad_year_and_seasons() {
        let teams = parse_team_index(TEAMS).unwrap();
        let roster = parse_roster(ROSTER, teams[0].clone()).unwrap();
        assert_eq!(roster.athletes.len(), 25);
        let first = &roster.athletes[0];
        assert_eq!(first.name, "Julian Aguilera");
        assert_eq!(first.roster_name, "Aguilera, Julian");
        assert_eq!(first.grad_year, GradYear::CO2027);
        assert_eq!(first.gender, Gender::Boys);
        assert_eq!(first.athlete_id, "14399169");
        assert!(first
            .profile_url
            .ends_with("/athletes/14399169-julian-aguilera"));
        // This athlete's roster row carries no season flags (matched athlete, no imported results).
        assert!(first.sports().is_empty());
        let all_sports = roster
            .athletes
            .iter()
            .find(|athlete| athlete.roster_name.starts_with("Altamirano"))
            .expect("Altamirano row present in fixture");
        assert_eq!(
            all_sports.sports(),
            vec![Sport::IndoorTrack, Sport::OutdoorTrack, Sport::CrossCountry]
        );
    }

    #[test]
    fn roster_entities_are_canonical_and_source_independent() {
        let teams = parse_team_index(TEAMS).unwrap();
        let roster = parse_roster(ROSTER, teams[0].clone()).unwrap();
        let site = Site::for_state("WI").unwrap();
        let (school, athletes, teams_out) =
            roster_entities(&roster, "WI", SchoolYear(2026), "2026-09-20", &site);
        assert_eq!(school.name, "Abbotsford");
        assert_eq!(school.city.as_deref(), Some("Abbotsford"));
        assert!(!athletes.is_empty());
        assert!(teams_out.len() >= 2, "indoor/outdoor/XC team variants");
        let aguilera = athletes
            .iter()
            .find(|athlete| athlete.canonical_name == "Julian Aguilera")
            .unwrap();
        assert_eq!(aguilera.grad_year, GradYear::CO2027);
        // Grade observed on a 2026-27 roster is 12 for a 2027 graduate.
        let observation = aguilera.observed_grades.first().unwrap();
        assert_eq!(observation.grade.get(), 12);
        assert_eq!(observation.grad_year(), GradYear::CO2027);
        assert_eq!(
            aguilera.id,
            CanonicalAthlete::mint(
                &school.id,
                "Julian Aguilera",
                GradYear::CO2027,
                Gender::Boys
            )
        );
    }

    #[test]
    fn malformed_html_fails_loudly() {
        assert!(parse_team_index("<html><body>no rows</body></html>").is_err());
        let teams = parse_team_index(TEAMS).unwrap();
        let roster = parse_roster("<html></html>", teams[0].clone()).unwrap();
        assert!(
            roster.athletes.is_empty(),
            "empty roster is data, not an error"
        );
    }
}
