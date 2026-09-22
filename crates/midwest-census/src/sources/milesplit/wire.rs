//! The site registry and the HTML shapes a team index and a roster are read into.
use census_domain::model::{Gender, GradYear};
use census_domain::UsJurisdiction;

/// One MileSplit state site: the jurisdiction it serves.
///
/// The jurisdiction is the validated key — every journal phase, report row and source id that needs
/// a state reads its code from here, so a mistyped state string cannot reach a request — and the
/// host is derived from that code rather than listed a second time (see [`Site::host`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Site {
    jurisdiction: UsJurisdiction,
}

impl Site {
    /// The site for one jurisdiction.
    pub const fn for_jurisdiction(jurisdiction: UsJurisdiction) -> Site {
        Site { jurisdiction }
    }

    /// The host serving this jurisdiction: `<lowercased code>.milesplit.com`.
    ///
    /// Evidence: `research/sources/milesplit-national/samples/state-subdomain-probe.txt` (captured
    /// 2026-09-22, curl 8.21.0): all fifty-one `<code>.milesplit.com/teams` hosts answered `200`
    /// with that same host as the effective URL, so each site is the code's own subdomain. The three
    /// slugs that answered `500` (`newengland`, `dcmdva`, `national`) serve no single jurisdiction
    /// and so have no variant in [`UsJurisdiction`]; deriving the host means a jurisdiction can only
    /// ever address its own site.
    pub fn host(&self) -> String {
        format!("{}.milesplit.com", self.code().to_ascii_lowercase())
    }

    /// The jurisdiction's USPS code — the string form used in source ids, journal phases and report
    /// rows.
    pub const fn code(&self) -> &'static str {
        self.jurisdiction.code()
    }

    /// The jurisdiction itself — the typed form canonical entities carry.
    pub const fn jurisdiction(&self) -> UsJurisdiction {
        self.jurisdiction
    }

    pub fn teams_url(&self) -> String {
        format!("https://{}/teams", self.host())
    }

    pub fn source_id(&self) -> String {
        format!("milesplit_{}", self.code().to_ascii_lowercase())
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
