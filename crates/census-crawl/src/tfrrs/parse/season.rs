//! The season and grade vocabulary the pages and their routes publish.
//!
//! The grade a row publishes is the source's own token (`SR`, `JR`, `SO`, `FR`, `8`, `7`, `6`)
//! and the *filter* the page was requested with uses the same vocabulary, so both share
//! [`YearToken`].

use census_domain::model::{Grade, SchoolYear, Sport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Season {
    pub year: i16,
    pub sport: Option<Sport>,
    /// `2022-23 Indoor` leads with the school year it belongs to; `2026 Indoor` with its calendar
    /// year. The flag keeps the two spellings from disagreeing about which August opened the year.
    pub school_year_label: bool,
}

impl Season {
    /// The school year a grade published for this season belongs to, or `None` when neither the page
    /// nor its route names a sport (a grade with no school year is not an observation).
    ///
    /// Cross country runs in the autumn of its own calendar year; indoor and outdoor track run in
    /// the winter and spring of the school year that opened the previous August. The month passed to
    /// [`SchoolYear::containing`] is what applies that boundary, so the rule lives in the domain and
    /// this file only picks the month.
    pub fn school_year(self) -> Option<SchoolYear> {
        if self.school_year_label {
            return SchoolYear::containing(self.year, 10);
        }
        let month = match self.sport? {
            Sport::CrossCountry => 10,
            Sport::IndoorTrack => 3,
            Sport::OutdoorTrack => 5,
        };
        SchoolYear::containing(self.year, month)
    }

    /// This season with a route's sport filled in when the page's own label names none.
    pub fn with_sport(self, sport: Option<Sport>) -> Self {
        Self {
            sport: self.sport.or(sport),
            ..self
        }
    }
}

/// The sport a team or list route segment names.
///
/// `xc` is unambiguous. `tf` is not: the same track route serves indoor and outdoor, so only the
/// page's season label can say which one a track page publishes.
pub fn sport_from_route(segment: &str) -> Option<Sport> {
    match segment {
        "xc" => Some(Sport::CrossCountry),
        _ => None,
    }
}

/// The source's own grade vocabulary, shared by the `Year` column and the `year=` filter.
///
/// `8`, `7` and `6` are published for middle-school competitors. They parse — the site's filter
/// offers them — but no canonical [`Grade`] exists below 9, so [`YearToken::grade`] reports `None`
/// and the caller counts the row instead of inventing a grade for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YearToken {
    Senior,
    Junior,
    Sophomore,
    Freshman,
    Eighth,
    Seventh,
    Sixth,
}

impl YearToken {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_uppercase().as_str() {
            "SR" => Some(YearToken::Senior),
            "JR" => Some(YearToken::Junior),
            "SO" => Some(YearToken::Sophomore),
            "FR" => Some(YearToken::Freshman),
            "8" => Some(YearToken::Eighth),
            "7" => Some(YearToken::Seventh),
            "6" => Some(YearToken::Sixth),
            _ => None,
        }
    }

    /// The canonical grade this token denotes, when the domain has one.
    pub fn grade(self) -> Option<Grade> {
        let number = match self {
            YearToken::Senior => 12,
            YearToken::Junior => 11,
            YearToken::Sophomore => 10,
            YearToken::Freshman => 9,
            YearToken::Eighth | YearToken::Seventh | YearToken::Sixth => return None,
        };
        Grade::new(number)
    }
}

/// Read a season label (`2026 NHIAA DII Cross Country`, `2026 Outdoor`, `2022-23 Indoor`).
///
/// The sport is only what the label *names*: `2026 NHIAA Division II` states a year and nothing
/// else, and a caller that knows the sport from the route fills it in with [`Season::with_sport`]
/// rather than this function guessing between indoor and outdoor.
pub fn season_from_label(label: &str) -> Option<Season> {
    let lowered = label.to_ascii_lowercase();
    let year = year_in(&lowered)?;
    let school_year_label = lowered.contains(&format!("{year}-"));
    let sport = if lowered.contains("cross country") || lowered.contains("xc") {
        Some(Sport::CrossCountry)
    } else if lowered.contains("indoor") {
        Some(Sport::IndoorTrack)
    } else if lowered.contains("outdoor") {
        Some(Sport::OutdoorTrack)
    } else {
        None
    };
    Some(Season {
        year,
        sport,
        school_year_label,
    })
}

/// The first four-digit run in a label, which is the year every season label leads with.
fn year_in(label: &str) -> Option<i16> {
    let mut digits = String::with_capacity(4);
    for ch in label.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            if digits.len() == 4 {
                return digits.parse::<i16>().ok();
            }
        } else {
            digits.clear();
        }
    }
    None
}
