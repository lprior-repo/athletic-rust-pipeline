use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use census_crawl::{ks, AdapterReport};
use census_domain::model::{CanonicalCoach, CanonicalSchool};
use census_store::Table;
use serde::Serialize;

use super::{assert_rollup, common, context, golden, golden_case, seeded, stem_of, OBSERVED_ON};

#[derive(Serialize)]
struct SchoolsRun<'a> {
    report: &'a AdapterReport,
    schools: &'a [CanonicalSchool],
    coaches: &'a [CanonicalCoach],
}

#[derive(Serialize)]
struct DirectoryRow {
    id: u64,
    identifier: String,
    school_name: String,
    mailing_city: String,
    class: Option<String>,
    enrollment: Option<u32>,
    web_site: Option<String>,
    ad_name: Option<String>,
    ad_email: Option<String>,
    ad_cell: Option<String>,
    principal_name: Option<String>,
}

impl DirectoryRow {
    fn of(record: &ks::KshsaaRecord) -> Self {
        let ks::KshsaaRecord {
            id,
            identifier,
            school_name,
            mailing_city,
            class,
            enrollment,
            web_site,
            ad_name,
            ad_email,
            ad_cell,
            principal_name,
        } = record;
        Self {
            id: *id,
            identifier: identifier.clone(),
            school_name: school_name.clone(),
            mailing_city: mailing_city.clone(),
            class: class.clone(),
            enrollment: *enrollment,
            web_site: web_site.clone(),
            ad_name: ad_name.clone(),
            ad_email: ad_email.clone(),
            ad_cell: ad_cell.clone(),
            principal_name: principal_name.clone(),
        }
    }
}

#[derive(Serialize)]
struct KshsaaFacts {
    records: Vec<DirectoryRow>,
    schools: Vec<CanonicalSchool>,
    ads: Vec<CanonicalCoach>,
}

#[test]
fn ks_fixtures_match_the_golden_corpus() -> Result<()> {
    let paths = common::fixtures("ks")?;
    let api = "https://kshsaa-api.kshsaa.org/directory/search/name/a/";
    let mut cases = BTreeMap::new();
    for path in &paths {
        let file = common::file_name(path)?;
        let stem = stem_of(&file);
        let body = common::fixture("ks", &file)?;
        let records = ks::parse_records(&body)?;
        let mut schools = Vec::new();
        let mut ads = Vec::new();
        for record in &records {
            let Some((school, school_id)) = ks::parse_school(record, api, OBSERVED_ON) else {
                bail!(
                    "{}: every fixture record has a school name",
                    record.identifier
                );
            };
            schools.push(school);
            if let Some(ad) = ks::parse_ad_coach(record, &school_id, api, OBSERVED_ON) {
                ads.push(ad);
            }
        }
        let facts = KshsaaFacts {
            records: records.iter().map(DirectoryRow::of).collect(),
            schools,
            ads,
        };
        let (name, digest) = golden_case(&format!("ks__{stem}"), &facts)?;
        cases.insert(name, digest);
    }
    assert_rollup("ks", cases, paths.len())
}

#[test]
fn ks_collect_from_a_seeded_cache_matches_golden() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
    let url = "https://kshsaa-api.kshsaa.org/directory/search/name/a/";
    let dir = tempfile::tempdir().context("temp dir")?;
    let (store, fetcher) = seeded(
        dir.path(),
        &[(url, &common::fixture("ks", "kshsaa_directory_a.json")?)],
    )?;
    let options = ks::Options {
        limit: None,
        refresh: false,
        observed_on: OBSERVED_ON.to_string(),
        states: Vec::new(),
        school_names: Vec::new(),
    };
    let report = ks::collect(&context(&fetcher, &store), &options).await?;

    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
    anyhow::ensure!(
        report.requests == 0,
        "the seeded cache answers the one request — left={:?} right={:?}",
        &report.requests,
        &0
    );
    anyhow::ensure!(
        report.from_cache == 1,
        "the directory response is cached — left={:?} right={:?}",
        &report.from_cache,
        &1
    );
    {
        let left_value = &(report.rows, schools.len(), coaches.len());
        let right_value = &(5, 5, 5);
        anyhow::ensure!(
            left_value == right_value,
            "five records mint five schools and five athletic directors: {report:?} — left={left_value:?} right={right_value:?}"
        );
    }
    golden::assert_golden(
        "ks__collect_directory_a",
        &SchoolsRun {
            report: &report,
            schools: &schools,
            coaches: &coaches,
        },
    )
        })
}
