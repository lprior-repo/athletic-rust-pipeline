use census_domain::model::{Gender, GradYear};
use census_domain::UsJurisdiction;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Site {
    jurisdiction: UsJurisdiction,
}

impl Site {
    pub const fn for_jurisdiction(jurisdiction: UsJurisdiction) -> Site {
        Site { jurisdiction }
    }

    pub fn host(&self) -> String {
        format!("{}.milesplit.com", self.code().to_ascii_lowercase())
    }

    pub const fn code(&self) -> &'static str {
        self.jurisdiction.code()
    }

    pub const fn jurisdiction(&self) -> UsJurisdiction {
        self.jurisdiction
    }

    pub fn teams_url(&self) -> String {
        format!("https://{}/teams", self.host())
    }

    pub fn results_url(&self, season: Season, year: u16, page: u32) -> String {
        format!(
            "https://{}/results?season={}&level=hs&year={year}&page={page}",
            self.host(),
            season.code()
        )
    }

    pub fn source_id(&self) -> String {
        format!("milesplit_{}", self.code().to_ascii_lowercase())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    CrossCountry,
    Indoor,
    Outdoor,
}

impl Season {
    pub const fn code(self) -> &'static str {
        match self {
            Season::CrossCountry => "cc",
            Season::Indoor => "indoor",
            Season::Outdoor => "outdoor",
        }
    }

    pub const ALL: [Season; 3] = [Season::CrossCountry, Season::Indoor, Season::Outdoor];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetRef {
    pub meet_id: String,
    pub name: String,
    pub date: Option<String>,
    pub venue: String,
    pub results_url: String,
}

impl MeetRef {
    pub fn meet_url(&self) -> String {
        self.results_url
            .strip_suffix("/results")
            .map_or(self.results_url.as_str(), |value| value)
            .to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultSetRef {
    pub site: Site,
    pub meet_id: String,
    pub rsid: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MeetResultFile {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "isMeetPro", default)]
    pub is_meet_pro: i64,
    #[serde(default)]
    pub inline: bool,
}

impl MeetResultFile {
    pub fn raw_url(&self, results_url: &str) -> String {
        let base = results_url.trim_end_matches('/');
        if self.inline {
            return base.to_string();
        }
        format!("{base}/{}/raw", self.id)
    }
}

impl ResultSetRef {
    pub fn parse(url: &str) -> Option<ResultSetRef> {
        let trimmed = url.trim();
        let (host, path) = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))?
            .split_once('/')?;
        let jurisdiction = UsJurisdiction::from_code(host.strip_suffix(".milesplit.com")?)?;
        result_set(host, path, Site::for_jurisdiction(jurisdiction))
    }

    pub fn parse_with_jurisdiction(
        url: &str,
        jurisdiction: UsJurisdiction,
    ) -> Option<ResultSetRef> {
        let trimmed = url.trim();
        let (host, path) = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))?
            .split_once('/')?;
        if host == "www.milesplit.com" {
            return result_set(host, path, Site::for_jurisdiction(jurisdiction));
        }
        if !host.ends_with(".milesplit.com") {
            return None;
        }
        let jurisdiction = UsJurisdiction::from_code(host.strip_suffix(".milesplit.com")?)?;
        result_set(host, path, Site::for_jurisdiction(jurisdiction))
    }
}

fn result_set(host: &str, path: &str, site: Site) -> Option<ResultSetRef> {
    let segments: Vec<&str> = path.split('/').collect();
    let last = *segments.last()?;
    let rsid = if last.eq_ignore_ascii_case("raw") {
        after(&segments, "results")?
    } else if last.eq_ignore_ascii_case("results") {
        "0".to_string()
    } else {
        return None;
    };
    let meet_id = after(&segments, "meets")?;
    Some(ResultSetRef {
        site,
        meet_id,
        rsid,
        url: format!("https://{host}/{}", segments.join("/")),
    })
}

fn after(segments: &[&str], label: &str) -> Option<String> {
    let position = segments.iter().position(|segment| *segment == label)?;
    let segment = segments.get(position.checked_add(1)?)?;
    let digits: String = segment
        .chars()
        .take_while(|ch: &char| ch.is_ascii_digit())
        .collect::<String>();
    (!digits.is_empty()).then_some(digits)
}
