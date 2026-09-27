
use census_domain::model::{Grade, SchoolYear, Sport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Season {
    pub year: i16,
    pub sport: Option<Sport>,
    pub school_year_label: bool,
}

impl Season {
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

    pub fn with_sport(self, sport: Option<Sport>) -> Self {
        Self {
            sport: self.sport.or(sport),
            ..self
        }
    }
}

pub fn sport_from_route(segment: &str) -> Option<Sport> {
    match segment {
        "xc" => Some(Sport::CrossCountry),
        _ => None,
    }
}

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
