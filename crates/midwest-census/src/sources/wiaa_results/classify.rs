use census_domain::model::{CompetitionLevel, SchoolYear, Sport};

/// Classify an artifact by what the platform can do with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactFormat {
    HytekHtml,
    HytekText,
    RaceDay,
    /// A PDF release: read through `pdftotext -layout`, which preserves the report's columns.
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

/// Decide how an artifact will be read, from its extension and (optionally) its body.
pub fn artifact_format(extension: &str, body: Option<&str>) -> ArtifactFormat {
    match extension {
        "htm" | "html" => {
            let body = body.unwrap_or_default();
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

/// Which school year a meet date falls in.
///
/// A track season runs inside one school year (`2025-06-06` → 2024-25); a cross-country season opens
/// the next one (`2025-10-25` → 2025-26). Files that publish no date at all fall back to the archive
/// year with the sport's start month, which is why that fallback is documented rather than hidden.
///
/// `None` when that year is one no season may open in: the domain bounds them, so a caller refuses
/// the file instead of filing its rows under a year no source published.
pub fn school_year_for(date: &str, sport: Sport, archive_year: i16) -> Option<SchoolYear> {
    let year = date
        .get(..4)
        .and_then(|value| value.parse::<i16>().ok())
        .unwrap_or(archive_year);
    let month = date.get(5..7).and_then(|value| value.parse::<u8>().ok());
    match (month, sport) {
        (Some(month), _) => SchoolYear::containing(year, month),
        (None, Sport::CrossCountry) => SchoolYear::containing(year, 10),
        (None, _) => SchoolYear::containing(year, 6),
    }
}

/// Competition level from the meet name the result file publishes.
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
