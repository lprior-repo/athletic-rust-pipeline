use crate::common;

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{ensure, Result};
use census_crawl::net::Fetcher;
use census_crawl::wiaa_results;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, Sport,
};
use census_store::{Store, Table};

#[path = "../common/capture_cache.rs"]
mod capture_cache;

use super::fixture_builders;
use super::ohsaa_wiaa_builders;

#[derive(Default)]
pub struct Corpus {
    pub schools: Vec<CanonicalSchool>,
    pub teams: Vec<CanonicalTeam>,
    pub coaches: Vec<CanonicalCoach>,
    pub athletes: Vec<CanonicalAthlete>,
    pub meets: Vec<CanonicalMeet>,
    pub events: Vec<CanonicalEvent>,
    pub performances: Vec<CanonicalPerformance>,
    pub artifacts: Vec<ResultArtifact>,
    pub expected: ExpectedEntities,
    pub published_results: Vec<(String, census_crawl::result_file::ParsedMeet, Sport)>,
}

pub struct ResultArtifact {
    pub file: String,
    pub url: String,
    pub page: usize,
    pub year: i16,
    pub label: &'static str,
}

#[derive(Default, Clone)]
pub struct ExpectedEntities {
    pub meets: BTreeSet<String>,
    pub events: BTreeSet<String>,
    pub teams: BTreeSet<String>,
    pub athletes: BTreeSet<String>,
    pub performances: BTreeSet<String>,
    pub relay_results: BTreeSet<String>,
}

impl Corpus {
    pub fn build() -> Result<Self> {
        let mut corpus = Corpus::default();
        super::wiaa_results::wiaa_result_files(&mut corpus)?;
        ohsaa_wiaa_builders::wiaa_directory(&mut corpus)?;
        ohsaa_wiaa_builders::ohsaa_schools(&mut corpus)?;
        fixture_builders::milesplit_roster(&mut corpus)?;
        fixture_builders::athleticlive_meets(&mut corpus)?;
        fixture_builders::athleticlive_athletes(&mut corpus)?;
        ensure!(
            !corpus.artifacts.is_empty() && !corpus.expected.performances.is_empty(),
            "the corpus carries no result artifact or no performance"
        );
        Ok(corpus)
    }

    pub fn tables(&self) -> [(&'static str, usize, BTreeSet<String>); 7] {
        [
            (
                "schools",
                self.schools.len(),
                ids_of(&self.schools, |row| row.id.as_str()),
            ),
            (
                "teams",
                self.teams.len(),
                ids_of(&self.teams, |row| row.id.as_str()),
            ),
            (
                "coaches",
                self.coaches.len(),
                ids_of(&self.coaches, |row| row.id.as_str()),
            ),
            (
                "athletes",
                self.athletes.len(),
                ids_of(&self.athletes, |row| row.id.as_str()),
            ),
            (
                "meets",
                self.meets.len(),
                ids_of(&self.meets, |row| row.id.as_str()),
            ),
            (
                "events",
                self.events.len(),
                ids_of(&self.events, |row| row.id.as_str()),
            ),
            (
                "performances",
                self.performances.len(),
                ids_of(&self.performances, |row| row.id.as_str()),
            ),
        ]
    }

    pub fn merge_report(&self) -> Vec<(String, usize)> {
        self.tables()
            .into_iter()
            .map(|(table, rows, ids)| (format!("{table}_merged"), rows.saturating_sub(ids.len())))
            .collect()
    }

    pub fn append(&self, store: &Store) -> Result<()> {
        store.append_many(Table::Schools, &self.schools)?;
        store.append_many(Table::Teams, &self.teams)?;
        store.append_many(Table::Coaches, &self.coaches)?;
        store.append_many(Table::Athletes, &self.athletes)?;
        store.append_many(Table::Meets, &self.meets)?;
        store.append_many(Table::Events, &self.events)?;
        store.append_many(Table::Performances, &self.performances)?;
        Ok(())
    }

    pub fn expected_counts(&self, with_results: bool) -> Vec<(&'static str, usize)> {
        self.tables()
            .into_iter()
            .map(|(table, _, mut ids)| {
                if with_results {
                    match table {
                        "athletes" => ids.extend(self.expected.athletes.iter().cloned()),
                        "meets" => ids.extend(self.expected.meets.iter().cloned()),
                        "events" => ids.extend(self.expected.events.iter().cloned()),
                        "teams" => ids.extend(self.expected.teams.iter().cloned()),
                        "performances" => ids.extend(self.expected.performances.iter().cloned()),
                        _ => {}
                    }
                }
                (table, ids.len())
            })
            .collect()
    }
}

impl ExpectedEntities {
    pub fn absorb_into(&self, target: &mut ExpectedEntities) {
        target.meets.extend(self.meets.iter().cloned());
        target.events.extend(self.events.iter().cloned());
        target.teams.extend(self.teams.iter().cloned());
        target.athletes.extend(self.athletes.iter().cloned());
        target
            .performances
            .extend(self.performances.iter().cloned());
        target
            .relay_results
            .extend(self.relay_results.iter().cloned());
    }
}

pub fn seed_archives(fetcher: &Fetcher, corpus: &Corpus) -> Result<()> {
    for (page, (url, _)) in wiaa_results::ARCHIVES.iter().enumerate() {
        let listing: Vec<&ResultArtifact> = corpus
            .artifacts
            .iter()
            .filter(|artifact| artifact.page == page)
            .collect();
        seed(fetcher.cache_dir(), url, &archive_page(url, &listing))?;
    }
    for artifact in &corpus.artifacts {
        seed(
            fetcher.cache_dir(),
            &artifact.url,
            &common::fixture("wiaa_results", &artifact.file)?,
        )?;
    }
    Ok(())
}

fn archive_page(url: &str, artifacts: &[&ResultArtifact]) -> String {
    let mut body = format!("<h3><strong>{url}</strong></h3>\n<ul>\n");
    for artifact in artifacts {
        let href = artifact
            .url
            .strip_prefix("https://www.wiaawi.org")
            .map_or(artifact.url.as_str(), |value| value);
        let ResultArtifact { year, label, .. } = artifact;
        body.push_str(&format!(
            "  <li>{year} - <a href=\"{href}\">{label}</a></li>\n"
        ));
    }
    body.push_str("</ul>\n");
    body
}

fn seed(cache: &Path, url: &str, body: &str) -> Result<()> {
    capture_cache::seed_response(cache, url, body.as_bytes(), super::constants::WIAA_CAPTURED_AT, capture_cache::Metadata {
        content_type: Some(if url.ends_with(".txt") { "text/plain" } else { "text/html" }),
        ..capture_cache::Metadata::default()
    })
}

fn ids_of<T>(rows: &[T], id: impl Fn(&T) -> &str) -> BTreeSet<String> {
    rows.iter().map(|row| id(row).to_string()).collect()
}
