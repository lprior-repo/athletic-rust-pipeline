//! MileSplit HTML adapter — the state team index, the graded roster, and the `/raw` result set.
//!
//! Only robots-permitted, server-rendered surfaces are used. Each route is **one request per unit**,
//! and none of the three paginates:
//!
//! | route | unit | requests per unit | rows per request (measured) |
//! |---|---|---|---|
//! | `/{teams}` | one state | 1 | 26,562 teams across the 51 jurisdictions; OH 977, WI 597 (`research/sources/milesplit-national/samples/teams-count-sweep.tsv`, one body per state, HTTP 200) |
//! | `/{teams}/<id>-<slug>/roster` | one team | 1 | 319 rows for the OH Mason capture, 96 of them Class of 2027 (`samples/roster-oh-mason.html`); every row publishes `column-grad-year` |
//! | `/meets/<id>/results/<RSID>/raw` | one result set | 1 | 80 rows in 2 sections for `RSID 1321880` (`samples/raw-oh-770621-rs1321880.txt`, 47,564 B, HTTP 200) |
//!
//! Those are the numbers a plan budgets with: a roster sweep costs one request per team in the
//! fetched index, and the result-set walk costs exactly the `/raw` URLs it is given. The census
//! driver is `census::sweep` for the first two routes and `milesplit::collect_result_sets`
//! (`results::collect`) for the third.
//!
//! `/api/` — and `/rankings`, `/virtual-meets`, `/contact` — are `Disallow`ed for every agent in
//! every captured `robots.txt`, on every host (`samples/robots-oh-milesplit.txt:7`), so no route here
//! ever requests them or the `api.prod.milesplit.com` host. That is why results are read from
//! `/raw`: the formatted view (`…/results/<RSID>/formatted`) is a JS shell that renders **0** rows
//! and builds its table from `v1/meets/<MeetID>/performances` behind `/api/`, so the `/raw` URL is
//! operator-supplied rather than discovered, and [`wire::ResultSetRef`] rejects every other shape.
//!
//! Layout: `wire` holds the site registry and the parsed shapes, `parse`/`normalize` the team-index
//! and roster readers and the entities they mint, `raw`/`raw_rows` the fixed-width result reader and
//! its column map, and `map`/`results` the canonical mapping, the journal and the report of the
//! result-set route.

mod fetch;
mod map;
mod normalize;
mod parse;
mod raw;
mod raw_rows;
mod results;
mod wire;

pub use fetch::{fetch_result_set, fetch_roster, fetch_team_index};
pub use normalize::roster_entities;
pub use parse::{parse_roster, parse_team_index};
pub use raw::{parse_raw, RawPage};
pub use results::{collect as collect_result_sets, ResultSetOptions};
pub use wire::{ResultSetRef, Roster, RosterAthlete, Site, TeamRef};

// The moved bodies reach these through `super::*`; the test module is the only reader.
#[cfg(test)]
use census_domain::model::{CanonicalAthlete, Gender, GradYear, SchoolYear, Sport};

#[cfg(test)]
mod tests;
