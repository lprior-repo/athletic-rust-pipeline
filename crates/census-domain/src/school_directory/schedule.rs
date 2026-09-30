use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

use super::DirectoryError;

pub const MIN_YEAR: i32 = 1900;

pub const MAX_YEAR: i32 = 2200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Month(u8);

impl Month {
    pub const JANUARY: Self = Self(1);
    pub const APRIL: Self = Self(4);
    pub const JULY: Self = Self(7);
    pub const SEPTEMBER: Self = Self(9);
    pub const OCTOBER: Self = Self(10);

    pub fn new(number: u8) -> Result<Self, DirectoryError> {
        if (1..=12).contains(&number) {
            Ok(Self(number))
        } else {
            Err(DirectoryError::MalformedMonth { value: number })
        }
    }

    pub const fn number(self) -> u8 {
        self.0
    }
}

impl fmt::Display for Month {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:02}", self.0)
    }
}

impl Serialize for Month {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(self.0)
    }
}

impl<'de> Deserialize<'de> for Month {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u8::deserialize(deserializer)?;
        Self::new(value).map_err(DeError::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth {
    year: i32,
    month: Month,
}

impl YearMonth {
    pub fn new(year: i32, month: Month) -> Result<Self, DirectoryError> {
        if (MIN_YEAR..=MAX_YEAR).contains(&year) {
            Ok(Self { year, month })
        } else {
            Err(DirectoryError::MalformedYear { value: year })
        }
    }

    pub const fn year(self) -> i32 {
        self.year
    }

    pub const fn month(self) -> Month {
        self.month
    }

    const fn clamped(year: i32, month: Month) -> Self {
        Self {
            year: clamp_year(year),
            month,
        }
    }
}

const fn clamp_year(year: i32) -> i32 {
    if year < MIN_YEAR {
        MIN_YEAR
    } else if year > MAX_YEAR {
        MAX_YEAR
    } else {
        year
    }
}

impl fmt::Display for YearMonth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:04}-{}", self.year, self.month)
    }
}

impl FromStr for YearMonth {
    type Err = DirectoryError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let malformed = || DirectoryError::MalformedYearMonth {
            value: text.to_string(),
        };
        let mut segments = text.trim().split('-');
        let year = segments.next().ok_or_else(malformed)?;
        let month = segments.next().ok_or_else(malformed)?;
        if segments.next().is_some() {
            return Err(malformed());
        }
        let year: i32 = year.parse().map_err(|_| malformed())?;
        let month: u8 = month.parse().map_err(|_| malformed())?;
        Self::new(year, Month::new(month).map_err(|_| malformed())?)
    }
}

impl Serialize for YearMonth {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for YearMonth {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::from_str(&text).map_err(DeError::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Parity {
    Even,
    Odd,
}

impl Parity {
    pub const fn of(year: i32) -> Self {
        if year.rem_euclid(2) == 0 {
            Self::Even
        } else {
            Self::Odd
        }
    }

    pub const fn matches(self, year: i32) -> bool {
        matches!(
            (self, Parity::of(year)),
            (Self::Even, Self::Even) | (Self::Odd, Self::Odd)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cadence {
    Yearly { month: Month },
    Biennial { parity: Parity },
    Semester { months: [Month; 2] },
    Quarterly { months: [Month; 4] },
}

impl Cadence {
    pub fn window_at_or_before(self, at: YearMonth) -> YearMonth {
        match self {
            Self::Yearly { month } => window_of_month(month, at),
            Self::Biennial { parity } => {
                let year = if parity.matches(at.year()) {
                    at.year()
                } else {
                    at.year().saturating_sub(1)
                };
                YearMonth::clamped(year, Month::JANUARY)
            }
            Self::Semester { months } => months
                .iter()
                .map(|month| window_of_month(*month, at))
                .max()
                .unwrap_or(at),
            Self::Quarterly { months } => months
                .iter()
                .map(|month| window_of_month(*month, at))
                .max()
                .unwrap_or(at),
        }
    }

    pub fn next_window_after(self, at: YearMonth) -> YearMonth {
        match self {
            Self::Yearly { month } => window_after_month(month, at),
            Self::Biennial { parity } => {
                let offset = if parity.matches(at.year()) { 2 } else { 1 };
                YearMonth::clamped(at.year().saturating_add(offset), Month::JANUARY)
            }
            Self::Semester { months } => months
                .iter()
                .map(|month| window_after_month(*month, at))
                .min()
                .unwrap_or(at),
            Self::Quarterly { months } => months
                .iter()
                .map(|month| window_after_month(*month, at))
                .min()
                .unwrap_or(at),
        }
    }
}

fn window_of_month(month: Month, at: YearMonth) -> YearMonth {
    if month.number() <= at.month().number() {
        YearMonth::clamped(at.year(), month)
    } else {
        YearMonth::clamped(at.year().saturating_sub(1), month)
    }
}

fn window_after_month(month: Month, at: YearMonth) -> YearMonth {
    if month.number() > at.month().number() {
        YearMonth::clamped(at.year(), month)
    } else {
        YearMonth::clamped(at.year().saturating_add(1), month)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DueReason {
    NeverUpdated,
    WindowOpened,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateDecision {
    Due { reason: DueReason },
    NotDue { due: YearMonth },
}

pub fn decide(cadence: Cadence, last: Option<YearMonth>, now: YearMonth) -> UpdateDecision {
    match last {
        None => UpdateDecision::Due {
            reason: DueReason::NeverUpdated,
        },
        Some(last) => {
            if cadence.window_at_or_before(now) > cadence.window_at_or_before(last) {
                UpdateDecision::Due {
                    reason: DueReason::WindowOpened,
                }
            } else {
                UpdateDecision::NotDue {
                    due: cadence.next_window_after(now),
                }
            }
        }
    }
}

pub fn next_due(cadence: Cadence, last: Option<YearMonth>, now: YearMonth) -> YearMonth {
    match last {
        Some(last) if cadence.window_at_or_before(last) >= cadence.window_at_or_before(now) => {
            cadence.next_window_after(now)
        }
        _ => cadence.window_at_or_before(now),
    }
}
