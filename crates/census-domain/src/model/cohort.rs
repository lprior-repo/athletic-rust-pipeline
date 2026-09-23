use super::*;

// -------------------------------------------------------------------------------------------------
// Time, grade, cohort
// -------------------------------------------------------------------------------------------------

/// A school year, identified by its starting calendar year (2025 = "2025-26").
///
/// The year is private so a season can only be built by a constructor that bounds it: [`Self::new`]
/// for a year a source published and [`Self::containing`] for one a date implies. A header that
/// carries a two-digit year, a season id of zero, or a column read out of the wrong position is a
/// data error the caller has to decide about — never a stored season that later reads as one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SchoolYear(i16);

impl SchoolYear {
    /// Earliest opening year a season may carry: wide enough for every published meet, narrow enough
    /// to reject a value that is not a school year at all.
    pub const MIN_START_YEAR: i16 = 1900;

    /// Latest opening year a season may carry, the upper end of the same window.
    pub const MAX_START_YEAR: i16 = 2100;

    /// Build a season from its opening calendar year, e.g. `2025` for the 2025-26 season.
    ///
    /// `None` outside [`MIN_START_YEAR`](Self::MIN_START_YEAR)..=[`MAX_START_YEAR`](Self::MAX_START_YEAR):
    /// the constructor is the only way in from a bare number, so a caller holding one has to say
    /// what happens when it is not a season instead of storing it and finding out in a report.
    pub const fn new(start_year: i16) -> Option<Self> {
        if start_year < Self::MIN_START_YEAR || start_year > Self::MAX_START_YEAR {
            return None;
        }
        Some(Self(start_year))
    }

    /// The season a caller that names none of its own collects in: 2026-27, the season the census
    /// switched to a typed season.
    ///
    /// A constant rather than a fallible lookup, because the callers that need one - a CLI option, a
    /// request's default - have no error channel to reject a season with, and this value is inside
    /// [`MIN_START_YEAR`](Self::MIN_START_YEAR)..=[`MAX_START_YEAR`](Self::MAX_START_YEAR) by
    /// construction, so nothing about it can fail at that point.
    pub const DEFAULT: SchoolYear = SchoolYear(2026);

    /// The opening calendar year: 2025 for the 2025-26 season.
    pub const fn get(self) -> i16 {
        self.0
    }

    /// `"2025-26"`.
    pub fn short(self) -> String {
        // `%` is a truncating remainder, which `wrapping_rem` reproduces exactly (including for
        // negative years) without the `MIN % -1` panic; `saturating_add` keeps the value
        // well-defined at `i16::MAX` instead of panicking in debug builds.
        let end = self.0.saturating_add(1).wrapping_rem(100);
        format!("{}-{end:02}", self.0)
    }

    /// The school year that contains `date` for competition purposes (Aug 1 boundary).
    ///
    /// `None` when the opening year is not a season [`Self::new`] accepts, including the year below
    /// `i16::MIN` that a pre-August date would step back to: the caller either drops the observation
    /// or falls back to the season it is running, and neither path stores a year no source published.
    pub fn containing(year: i16, month: u8) -> Option<Self> {
        let opening = if month >= 8 {
            year
        } else {
            year.checked_sub(1)?
        };
        Self::new(opening)
    }
}

impl Default for SchoolYear {
    /// [`SchoolYear::DEFAULT`]: the season a caller that names none collects in.
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Grade level as observed in a specific school year: 9..=12.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Grade(u8);

impl Grade {
    pub fn new(grade: u8) -> Option<Self> {
        (9..=12).contains(&grade).then_some(Grade(grade))
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

impl fmt::Display for Grade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An absolute graduating class, e.g. `GradYear(2027)` = "Class of 2027".
///
/// This is the only sanctioned cohort key. The year is private so a class is either a placement a
/// source published ([`Self::new`], bounded) or the derivation [`Self::of`] makes from one grade
/// observation — a caller cannot hand the graph a class nothing observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GradYear(i16);

impl GradYear {
    /// Class of 2027.
    pub const CO2027: GradYear = GradYear(2027);

    /// Raw graduation year, e.g. `2027`.
    pub const fn get(self) -> i16 {
        self.0
    }

    /// Earliest class the census places: an adult who graduated in 2020 is out of every cohort this
    /// graph counts, and a row claiming one is a parse error rather than a class.
    pub const MIN_YEAR: i16 = 2020;

    /// Latest class the census places, the upper end of the same window.
    pub const MAX_YEAR: i16 = 2040;

    /// The class a source published; `None` outside
    /// [`MIN_YEAR`](Self::MIN_YEAR)..=[`MAX_YEAR`](Self::MAX_YEAR).
    pub const fn new(year: i16) -> Option<Self> {
        if year < Self::MIN_YEAR || year > Self::MAX_YEAR {
            return None;
        }
        Some(Self(year))
    }

    /// `grade` observed during `school_year` implies `grad_year = start + (13 - grade)`.
    ///
    /// Grade 11 in 2025-26 -> 2027. Grade 12 in 2026-27 -> 2027. Grade 9 in 2025-26 -> 2029.
    pub fn of(grade: Grade, school_year: SchoolYear) -> Self {
        // In-domain inputs (4-digit years, grades 9..=12) stay far inside `i16`; saturating keeps
        // out-of-domain inputs well-defined instead of panicking in debug builds.
        Self(
            school_year
                .get()
                .saturating_add(13)
                .saturating_sub(i16::from(grade.get())),
        )
    }
}

impl fmt::Display for GradYear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A grade observation: grade + the school year it was observed in + where it came from.
///
/// Never collapse this into a bare `grade`, and never overwrite it when a newer observation
/// arrives: history is evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedGrade {
    pub grade: Grade,
    pub school_year: SchoolYear,
    pub source: SourceRef,
}

impl ObservedGrade {
    pub fn grad_year(&self) -> GradYear {
        GradYear::of(self.grade, self.school_year)
    }
}
