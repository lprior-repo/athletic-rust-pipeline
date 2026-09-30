use crate::coach_directories::parse::{parse_summary, DirectorySchool};
use crate::coach_directories::{map, summary_url};
use crate::net::{FetchOptions, Fetcher};
use crate::CrawlResult;
use census_domain::model::{SchoolId, Sport};
use indexmap::IndexMap;

pub fn sample_positions(len: usize) -> Vec<usize> {
    let step = len.saturating_div(4).max(1);
    (0..len).step_by(step).take(4).collect()
}

pub fn sample_rows<T: std::borrow::Borrow<DirectorySchool>>(rows: &[T]) -> Vec<&DirectorySchool> {
    sample_positions(rows.len())
        .into_iter()
        .filter_map(|position| rows.get(position).map(std::borrow::Borrow::borrow))
        .collect()
}

pub struct SampleResult {
    pub staff: usize,
    pub coaches: usize,
    pub sports: IndexMap<String, usize>,
}

pub async fn process_sampled_school(
    fetcher: &Fetcher,
    row: &DirectorySchool,
) -> CrawlResult<SampleResult> {
    let Some(short_code) = row.short_code.as_deref() else {
        return Ok(SampleResult {
            staff: 0,
            coaches: 0,
            sports: IndexMap::new(),
        });
    };
    if short_code.is_empty() {
        return Ok(SampleResult {
            staff: 0,
            coaches: 0,
            sports: IndexMap::new(),
        });
    }
    let summary_url = summary_url(short_code);
    let options = FetchOptions::default();
    let summary_outcome = fetcher.get(&summary_url, &options).await?;
    let summary = parse_summary(&summary_outcome.body)?;
    let school_id = SchoolId::mint("sch", &["survey", short_code]);
    let emission = map::coach_entities(
        &summary,
        &school_id,
        &summary_url,
        "2026-09-29",
        map::EmissionScope::Probe,
    )?;
    let mut sports: IndexMap<String, usize> = IndexMap::new();
    for entity in &emission.coaches {
        let sport_key = artifact_sport_key(entity.sport);
        let counter = sports.entry(sport_key).or_insert(0);
        *counter = counter.saturating_add(1);
    }
    Ok(SampleResult {
        staff: summary.staff.len(),
        coaches: emission.coaches.len(),
        sports,
    })
}

fn artifact_sport_key(sport: Option<Sport>) -> String {
    match sport {
        Some(Sport::IndoorTrack) | Some(Sport::OutdoorTrack) => "Track".to_string(),
        Some(Sport::CrossCountry) => "CrossCountry".to_string(),
        None => "AthleticDirector".to_string(),
    }
}
