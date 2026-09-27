use crate::common;

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, ensure, Context, Result};
use census_crawl::result_file::ParsedMeet;
use census_crawl::{hytek, raceday, wiaa_results};
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, GradYear, Grade, SourceIdentity, SourceNamespace, SourceRef,
    Sport,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;

use super::fixtures::{Corpus, ExpectedEntities, ResultArtifact};

pub fn wiaa_result_files(corpus: &mut Corpus) -> Result<()> {
    let mut parsed: Vec<(ResultArtifact, ParsedMeet, Sport)> = Vec::new();
    let mut schools: BTreeMap<String, CanonicalSchool> = BTreeMap::new();
    let mut labels: BTreeSet<String> = BTreeSet::new();
    for path in common::fixtures("wiaa_results")? {
        let file = common::file_name(&path)?;
        let (page, year, label) = archive_identity(&file)?;
        let body = common::fixture("wiaa_results", &file)?;
        let (_, extension) = file
            .rsplit_once('.')
            .with_context(|| format!("{file} carries no extension"))?;
        let sport = wiaa_results::ARCHIVES[page].1;
        let url = format!(
            "https://www.wiaawi.org/Portals/0/PDF/Results/{}/{year}/{file}",
            archive_segment(sport)
        );
        let source = SourceRef::new("wiaa_results", Some(url.clone()));
        let format = wiaa_results::artifact_format(extension, Some(&body));
        let meet = match format {
            wiaa_results::ArtifactFormat::HytekHtml => {
                hytek::parse(&hytek::lines_from_html(&body), source)
            }
            wiaa_results::ArtifactFormat::HytekText => {
                hytek::parse(&hytek::lines_from_text(&body), source)
            }
            wiaa_results::ArtifactFormat::RaceDay => Some(
                raceday::parse(&body, source, year)
                    .with_context(|| format!("{file} is not a RaceDay report"))?,
            ),
            other => bail!(
                "{file} is classified as {}, which this harness cannot read",
                other.as_str()
            ),
        }
        .with_context(|| format!("{file} yielded no meet as {}", format.as_str()))?;
        ensure!(
            !meet.events.is_empty(),
            "{file}: the parsed meet carries no events"
        );
        ensure!(
            meet.events.iter().any(|event| !event.rows.is_empty()),
            "{file}: the parser accepted no row in any event"
        );
        ensure!(
            meet.date.starts_with(&year.to_string()),
            "{file}: the parsed date {:?} is not in the archive year {year} this harness serves it \
             under",
            meet.date
        );
        for event in &meet.events {
            for row in &event.rows {
                if !row.school.trim().is_empty() {
                    labels.insert(row.school.trim().to_string());
                }
            }
        }
        parsed.push((
            ResultArtifact {
                file,
                url,
                page,
                year,
                label,
            },
            meet,
            sport,
        ));
    }

    for label in &labels {
        let (school, id) =
            CanonicalSchool::new(UsJurisdiction::Wisconsin, label, normalize_name(label));
        schools.insert(id.as_str().to_string(), school);
    }
    let index = SchoolIndex::from_schools(&schools.values().cloned().collect::<Vec<_>>());
    for label in &labels {
        ensure!(
            index.resolve(UsJurisdiction::Wisconsin, label).is_some(),
            "the published school label {label:?} does not resolve against a school minted from it"
        );
    }

    let mut expected = ExpectedEntities::default();
    for (artifact, meet, sport) in &parsed {
        let graded = expected_ids_for(&index, artifact, meet, *sport, &mut expected)?;
        ensure!(
            graded > 0,
            "{}: no row carried a grade, so the fixture contributes no performance",
            artifact.file
        );
    }
    corpus.schools.extend(schools.into_values());
    corpus.artifacts = parsed
        .iter()
        .map(|(artifact, ..)| ResultArtifact {
            file: artifact.file.clone(),
            url: artifact.url.clone(),
            page: artifact.page,
            year: artifact.year,
            label: artifact.label,
        })
        .collect();
    expected.absorb_into(&mut corpus.expected);
    Ok(())
}

fn archive_identity(file: &str) -> Result<(usize, i16, &'static str)> {
    match file {
        "d1boysstateresults-dash.htm" => Ok((0, 2025, "Division 1 - Dash")),
        "d1boysstateresults-dash.txt" => Ok((0, 2025, "Division 1 - Dash (text)")),
        "d1boysstateresults-sections.htm" => Ok((0, 2025, "Division 1 - Sections")),
        "seed-column-regional.htm" => Ok((0, 2025, "Division 3 Colfax Regional")),
        "trackside-regional.htm" => Ok((1, 2025, "Division 1 Badger Regional")),
        "racinesectionalb-finish-list.htm" => Ok((2, 2023, "Division 2 Racine Sectional")),
        other => bail!("{other} is not a known wiaa_results fixture"),
    }
}

fn archive_segment(sport: Sport) -> &'static str {
    match sport {
        Sport::CrossCountry => "Cross_Country",
        _ => "Track",
    }
}

fn expected_ids_for(
    index: &SchoolIndex,
    artifact: &ResultArtifact,
    meet: &ParsedMeet,
    sport: Sport,
    expected: &mut ExpectedEntities,
) -> Result<usize> {
    let url = &artifact.url;
    let expected_meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        &meet.name,
        &meet.date,
        wiaa_results::level_of(&meet.name),
    );
    expected.meets.insert(expected_meet.id.as_str().to_string());
    let school_year = wiaa_results::school_year_for(&meet.date, sport, artifact.year)
        .with_context(|| {
            format!(
                "{} (archive {}) names no school season",
                meet.date, artifact.year
            )
        })?;

    let mut graded = 0usize;
    for event in &meet.events {
        let expected_event = CanonicalEvent::new(
            &expected_meet.id,
            event.kind.clone(),
            event.gender,
            event.division.as_deref(),
            event.round.as_deref(),
        );
        expected
            .events
            .insert(expected_event.id.as_str().to_string());

        for (row_index, row) in event.rows.iter().enumerate() {
            let label = row.school.trim();
            if label.is_empty() {
                continue;
            }
            let Some((school_id, _)) = index.resolve(UsJurisdiction::Wisconsin, label) else {
                bail!(
                    "{}: the published school label {label:?} has no canonical school",
                    artifact.file
                );
            };
            expected.teams.insert(
                CanonicalTeam::mint(&school_id, sport, event.gender, school_year)
                    .as_str()
                    .to_string(),
            );
            let members: Vec<(Option<u8>, &str, Option<Grade>)> = if row.legs.is_empty() {
                vec![(None, row.name.as_str(), row.grade)]
            } else {
                row.legs
                    .iter()
                    .map(|leg| (Some(leg.position), leg.name.as_str(), leg.grade))
                    .collect()
            };
            for (leg_position, member, grade) in members {
                let Some(grade) = grade else { continue };
                if member.trim().is_empty() {
                    continue;
                }
                graded = graded.saturating_add(1);
                let grad_year = GradYear::of(grade, school_year);
                let round = event.round.as_deref().unwrap_or("<none>");
                let source_key = match leg_position {
                    Some(position) => {
                        format!("{url}:{}:{round}:{row_index}:leg{position}", event.label)
                    }
                    None => format!("{url}:{}:{round}:{row_index}", event.label),
                };
                let source = SourceIdentity::new(
                    SourceNamespace::Other("wiaa_result_row".to_string()),
                    source_key.clone(),
                );
                let athlete_id =
                    CanonicalAthlete::mint(&school_id, member, grad_year, event.gender, &source);
                expected.athletes.insert(athlete_id.as_str().to_string());
                expected.performances.insert(
                    CanonicalPerformance::mint(
                        &athlete_id,
                        &expected_meet.id,
                        &event.kind,
                        &expected_meet.date,
                        &source_key,
                    )
                    .as_str()
                    .to_string(),
                );
            }
        }
    }
    Ok(graded)
}
