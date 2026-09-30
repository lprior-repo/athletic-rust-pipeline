use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct SchoolYear(i16);

impl<'de> Deserialize<'de> for SchoolYear {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let start_year = i16::deserialize(deserializer)?;
        SchoolYear::new(start_year).ok_or(serde::de::Error::invalid_value(
            serde::de::Unexpected::Other("i16 value"),
            &format!(
                "school year in [{}, {}]",
                SchoolYear::MIN_START_YEAR,
                SchoolYear::MAX_START_YEAR
            )
            .as_str(),
        ))
    }
}

impl SchoolYear {
    pub const MIN_START_YEAR: i16 = 1900;

    pub const MAX_START_YEAR: i16 = 2100;

    pub const fn new(start_year: i16) -> Option<Self> {
        if start_year < Self::MIN_START_YEAR || start_year > Self::MAX_START_YEAR {
            return None;
        }
        Some(Self(start_year))
    }

    pub const DEFAULT: SchoolYear = SchoolYear(2026);

    pub const fn get(self) -> i16 {
        self.0
    }

    pub fn short(self) -> String {
        let end = self.0.saturating_add(1).wrapping_rem(100);
        format!("{}-{end:02}", self.0)
    }

    pub fn containing(year: i16, month: u8) -> Option<Self> {
        let opening = if month >= 8 {
            year
        } else {
            year.checked_sub(1)?
        };
        Self::new(opening)
    }

    pub fn from_date(date: &str) -> Option<Self> {
        let date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
        let year = i16::try_from(chrono::Datelike::year(&date)).ok()?;
        let month = u8::try_from(chrono::Datelike::month(&date)).ok()?;
        Self::containing(year, month)
    }
}

impl Default for SchoolYear {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Grade(u8);

impl<'de> Deserialize<'de> for Grade {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let grade = u8::deserialize(deserializer)?;
        Grade::new(grade).ok_or_else(|| {
            serde::de::Error::invalid_value(
                serde::de::Unexpected::Other("u8 value"),
                &"grade in [9, 12]",
            )
        })
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct GradYear(i16);

impl<'de> Deserialize<'de> for GradYear {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let year = i16::deserialize(deserializer)?;
        GradYear::new(year).ok_or(serde::de::Error::invalid_value(
            serde::de::Unexpected::Other("i16 value"),
            &format!(
                "grad year in [{}, {}]",
                GradYear::MIN_YEAR,
                GradYear::MAX_YEAR
            )
            .as_str(),
        ))
    }
}

impl GradYear {
    pub const CO2027: GradYear = GradYear(2027);

    pub const fn get(self) -> i16 {
        self.0
    }

    pub const MIN_YEAR: i16 = 2020;

    pub const MAX_YEAR: i16 = 2040;

    pub const fn new(year: i16) -> Option<Self> {
        if year < Self::MIN_YEAR || year > Self::MAX_YEAR {
            return None;
        }
        Some(Self(year))
    }

    pub fn of(grade: Grade, school_year: SchoolYear) -> Self {
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
