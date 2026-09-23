//! The site registry and the HTML shapes a team index and a roster are read into.
use census_domain::model::{Gender, GradYear};
use census_domain::UsJurisdiction;
use serde::Deserialize;

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

    /// The state site's results index: `GET /results?season=&level=&year=&page=N`.
    ///
    /// One request serves 50 meet rows (`samples/results-oh.html`, captured 2026-09-22: exactly 50
    /// `data-meet-id` attributes in one 187 KB body), which is why the meet census enumerates here
    /// rather than by walking meet pages: a state's whole season costs one request per fifty meets.
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

/// The season selector the results index publishes (`ddSeason`): cross country, indoor, or outdoor
/// track. `road` exists on the site and is deliberately absent here — a road race is not a
/// high-school track or cross-country meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    CrossCountry,
    Indoor,
    Outdoor,
}

impl Season {
    /// The `ddSeason` value this season selects.
    pub const fn code(self) -> &'static str {
        match self {
            Season::CrossCountry => "cc",
            Season::Indoor => "indoor",
            Season::Outdoor => "outdoor",
        }
    }

    /// Both track seasons, which is what a whole-season meet census asks for by default.
    pub const ALL: [Season; 3] = [Season::CrossCountry, Season::Indoor, Season::Outdoor];
}

/// One meet row of a state's results index.
///
/// The id is kept as text because it is a source object's own identifier, never a number this
/// program computes with; the date is optional because the row's day and its month bucket are read
/// separately and a row whose month bucket was not published has no date to claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetRef {
    pub meet_id: String,
    pub name: String,
    pub date: Option<String>,
    pub venue: String,
    pub results_url: String,
}

impl MeetRef {
    /// The meet's own page: the published results link without its `/results` suffix.
    pub fn meet_url(&self) -> String {
        self.results_url
            .strip_suffix("/results")
            .unwrap_or(&self.results_url)
            .to_string()
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

/// One `/raw` result set: the site that published it and the two provider ids that name it.
///
/// A result set is addressed by its URL because nothing cheaper names it. The meet's results page
/// is where the list comes from: it embeds `meetResultFiles` — every result file the meet has, with
/// each file's own id — and that is what [`MeetResultFile::raw_url`] turns into the `/raw` URL this
/// type reads. (An earlier reading of this file held that only the *first* result set was published
/// and that the list lived behind the robots-disallowed `/api/`; the capture in
/// `tests/fixtures/milesplit/oh_meet_770621_results.html:356` shows the whole list on the page
/// itself, which is what the discovery arm now uses.) This type checks a URL into a jurisdiction, a
/// `MeetID` and an `RSID`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultSetRef {
    pub site: Site,
    pub meet_id: String,
    pub rsid: String,
    pub url: String,
}

/// One result file a meet's results page lists, as `meetResultFiles[]` publishes it.
///
/// `is_meet_pro` is carried exactly as the page publishes it, without a reading of its own: a file
/// the platform marks as Pro is still attempted, and the fetch layer reports whatever status the
/// host answers with rather than this layer guessing that the file is gated.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MeetResultFile {
    /// The provider's own result-set id, the `RSID` of the `/raw` route.
    pub id: i64,
    /// The name the page gives the file (`Results`, `Section 2`, …).
    #[serde(default)]
    pub name: String,
    /// The page's own Pro marker, uninterpreted.
    #[serde(rename = "isMeetPro", default)]
    pub is_meet_pro: i64,
}

impl MeetResultFile {
    /// The `/raw` URL of this file under the results page that listed it.
    pub fn raw_url(&self, results_url: &str) -> String {
        format!("{}/{}/raw", results_url.trim_end_matches('/'), self.id)
    }
}

impl ResultSetRef {
    /// Read a published `/meets/<id>-<slug>/results/<rsid>/raw` URL.
    ///
    /// `None` for anything else — including a `/formatted` URL, which serves an empty JS shell
    /// rather than rows — so a mistyped or unserved entry is reported instead of requested. The
    /// jurisdiction comes from the host and is validated with [`UsJurisdiction::from_code`], so a
    /// URL can only ever address a host that serves the jurisdiction it names.
    pub fn parse(url: &str) -> Option<ResultSetRef> {
        let trimmed = url.trim();
        let (host, path) = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))?
            .split_once('/')?;
        let jurisdiction = UsJurisdiction::from_code(host.strip_suffix(".milesplit.com")?)?;
        let segments: Vec<&str> = path.split('/').collect();
        if !segments.last()?.eq_ignore_ascii_case("raw") {
            return None;
        }
        let meet_id = after(&segments, "meets")?;
        let rsid = after(&segments, "results")?;
        Some(ResultSetRef {
            site: Site::for_jurisdiction(jurisdiction),
            meet_id,
            rsid,
            url: format!("https://{host}/{}", segments.join("/")),
        })
    }
}

/// The digits that open the segment following `label`, e.g. `770621` after `meets` in
/// `/meets/770621-beaver-eastern-invite-2026/results/1321880/raw`.
fn after(segments: &[&str], label: &str) -> Option<String> {
    let position = segments.iter().position(|segment| *segment == label)?;
    let segment = segments.get(position.checked_add(1)?)?;
    let digits: String = segment
        .chars()
        .take_while(|ch: &char| ch.is_ascii_digit())
        .collect::<String>();
    (!digits.is_empty()).then_some(digits)
}
