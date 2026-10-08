use census_domain::model::{CompetitionLevel, SchoolYear, Sport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactFormat {
    HytekHtml,
    HytekText,
    RaceDay,
    Pdf,
    Unparsed,
}

impl ArtifactFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            ArtifactFormat::HytekHtml => "hytek_html",
            ArtifactFormat::HytekText => "hytek_text",
            ArtifactFormat::RaceDay => "raceday",
            ArtifactFormat::Pdf => "pdf",
            ArtifactFormat::Unparsed => "unparsed",
        }
    }
}

pub fn artifact_format(extension: &str, body: Option<&str>) -> ArtifactFormat {
    match extension {
        "htm" | "html" => {
            let body = body.map_or(Default::default(), core::convert::identity);
            if body.contains("RaceDay Scoring") || body.contains("data-display") {
                ArtifactFormat::RaceDay
            } else {
                ArtifactFormat::HytekHtml
            }
        }
        "txt" => ArtifactFormat::HytekText,
        "pdf" => ArtifactFormat::Pdf,
        _ => ArtifactFormat::Unparsed,
    }
}

pub fn school_year_for(date: &str, sport: Sport, archive_year: i16) -> Option<SchoolYear> {
    SchoolYear::from_date(date).or_else(|| {
        let year = match date {
            value if value.len() == 4 && value.bytes().all(|byte| byte.is_ascii_digit()) => value
                .parse::<i16>()
                .ok()
                .filter(|year| SchoolYear::new(*year).is_some()),
            _ => None,
        }
        .map_or(archive_year, core::convert::identity);
        match sport {
            Sport::CrossCountry => SchoolYear::containing(year, 10),
            Sport::IndoorTrack | Sport::OutdoorTrack => SchoolYear::containing(year, 6),
            Sport::Unknown => None,
        }
    })
}

pub fn level_of(name: &str) -> CompetitionLevel {
    let lowered = name.to_ascii_lowercase();
    if lowered.contains("state") {
        CompetitionLevel::State
    } else if lowered.contains("sectional") {
        CompetitionLevel::Sectional
    } else if lowered.contains("regional") {
        CompetitionLevel::Regional
    } else if lowered.contains("conference") || lowered.contains("invit") {
        CompetitionLevel::Invitational
    } else {
        CompetitionLevel::Unknown
    }
}
