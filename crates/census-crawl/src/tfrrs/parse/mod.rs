//! Pure parsing: TFRRS's server-rendered HTML in, parsed rows out.
//!
//! No I/O, no store access and no canonical ids: the runner (`super`) fetches the pages,
//! `super::map` turns these rows into canonical entities, and this tree owns only what the
//! served markup states. Two page shapes are read:
//!
//! * a **performance list** (`/lists/<id>/<slug>/<year>/<i|o>`, optionally `?year=<TOKEN>`)
//!   whose rows are `div.performance-list-row` chunks whose cells are `div[data-label]` — the
//!   cells are read by label, so a column the host adds later cannot shift the values this
//!   reader returns;
//! * a **team page** (`/teams/<tf|xc>/<slug>_<m|f>.html`) whose `ROSTER` table publishes `NAME`
//!   and `YEAR` per athlete plus the numeric athlete id inside each link.
//!
//! Both route spellings the host publishes are read the same way: the relative path a page
//! prints in its own navigation and the absolute URL its own links carry.
//!
//! Every reader here is tested against captures of the pages the host served, or byte-exact
//! excerpts of one (`tests/fixtures/tfrrs/`, `tests.rs`).
//!
//! Layout: `html` the markup primitives every reader shares, `route` the URL shapes, `season`
//! the season and grade vocabulary, `date` the published date, `mark` the mark and the markers
//! the host qualifies it with, `list`/`row` the performance-list page and one of its rows,
//! `team` the roster page.

mod date;
mod html;
mod list;
mod mark;
mod route;
mod row;
mod season;
mod team;

#[cfg(test)]
pub use date::published_date;
pub use date::PublishedDate;
pub use list::{parse_list_page, ParsedList, ParsedSection};
pub use mark::{clock_seconds, feet_inches_metres, ParsedMark};
pub use route::{
    jurisdiction_of_url, list_filter, parse_list_path, parse_team_path, ListPath, TeamPath,
};
pub use row::{ParsedAthlete, ParsedMeet, ParsedRow, ParsedTeam};
#[cfg(test)]
pub use season::season_from_label;
pub use season::{sport_from_route, YearToken};
pub use team::{parse_team_page, ParsedRoster, RosterAthlete};
