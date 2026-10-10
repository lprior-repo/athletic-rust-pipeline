use super::{CAPTURED, RETAINED};
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::Fetcher;
use crate::tssaa::{collect, Options, DIRECTORY_URL};
use crate::{AdapterContext, AdapterReport, Recording};
use census_domain::model::{CanonicalCoach, CanonicalSchool, SchoolYear};
use census_store::{Store, Table};
use std::collections::HashMap;
use std::time::Duration;

mod cases;
mod tenure;

struct FixtureRun {
    _root: tempfile::TempDir,
    store: Store,
    fetcher: Fetcher,
}

impl FixtureRun {
    fn new(directory: &[u8], detail: Option<&[u8]>) -> anyhow::Result<Self> {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path().join("store"))?;
        let fetcher = Fetcher::new(
            root.path().join("http"),
            None,
            Duration::from_millis(1),
            HashMap::new(),
            vec![],
        )?
        .with_offline(true);
        seed(&fetcher, DIRECTORY_URL, directory)?;
        if let Some(detail) = detail {
            seed(&fetcher, &format!("{DIRECTORY_URL}?id=157"), detail)?;
        }
        Ok(Self {
            _root: root,
            store,
            fetcher,
        })
    }

    async fn run(
        &self,
        options: &Options,
        recording: Option<&Recording>,
    ) -> anyhow::Result<AdapterReport> {
        let context = AdapterContext {
            fetcher: &self.fetcher,
            store: &self.store,
            refresh: false,
            school_year: SchoolYear::new(2026)
                .ok_or_else(|| anyhow::anyhow!("fixture school year is invalid"))?,
            observed_on: "2026-10-02".to_string(),
            performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-02", "%Y-%m-%d")?,
            recording,
        };
        Ok(collect(&context, options).await?)
    }

    fn schools(&self) -> anyhow::Result<Vec<CanonicalSchool>> {
        Ok(self.store.scan(Table::Schools)?)
    }

    fn coaches(&self) -> anyhow::Result<Vec<CanonicalCoach>> {
        let mut coaches = self.store.scan::<CanonicalCoach>(Table::Coaches)?;
        coaches.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(coaches)
    }
}

fn page_options() -> Options {
    Options {
        school_names: vec!["Page High School".to_string()],
        observed_on: "2026-10-02".to_string(),
        ..Options::default()
    }
}

fn seed(fetcher: &Fetcher, url: &str, body: &[u8]) -> anyhow::Result<()> {
    let key = Fetcher::key_for("GET", url, "");
    let (body_path, meta_path) = fetcher.cache_paths(&key);
    write_cache(
        &body_path,
        &meta_path,
        body,
        &CacheMeta {
        redirects: Vec::new(),
            representation: crate::net::RepresentationHeaders::default(),
            url: url.to_string(),
            response_url: None,
            method: "GET".to_string(),
            status: 200,
            content_digest: content_digest(body),
            bytes: body.len(),
            fetched_at: CAPTURED.to_string(),
            etag: None,
            last_modified: None,
            content_type: Some("text/html;charset=UTF-8".to_string()),
        },
    )?;
    Ok(())
}

#[test]
fn retained_page_collects_source_owned_postal_claims_with_capture_freshness(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(RETAINED, Some(RETAINED))?;
            let report = run.run(&page_options(), None).await?;
            check!(eq; (report.rows, report.errors, report.with_email), (1, 0, 11));
            let schools = run.schools()?;
            let school = schools.first().ok_or_else(|| anyhow::anyhow!("school"))?;
            check!(eq; school.name, "Page High School");
            check!(eq; school.state, Some(census_domain::UsJurisdiction::Tennessee));
            check!(eq; school.city.as_deref(), Some("Franklin"));
            check!(eq;
                school
                    .source_identities
                    .first()
                    .map(|owner| owner.id.as_str()),
                Some("157")
            );
            check!(eq;
                school.source_identities.first().map(|owner| &owner.namespace),
                Some(&census_domain::model::SourceNamespace::association_school("tssaa"))
            );
            check!(eq;
                school
                    .evidence
                    .first()
                    .map(|evidence| evidence.observed_on.as_str()),
                Some(CAPTURED)
            );
            assert_postal_claims(school)?;
            let coaches = run.coaches()?;
            check!(eq; coaches.len(), 11);
            check!(coaches.iter().all(|coach| coach.school == school.id
                && coach
                    .evidence
                    .iter()
                    .all(|evidence| evidence.observed_on == CAPTURED)));
            check!(eq; report.disposition, crate::CollectionDisposition::Complete);
            check!(eq; report.unfinished, Vec::<String>::new());
            let completions = run.store.journal_payloads(super::super::collect::JOURNAL)?;
            assert_completion(&completions, school)?;
            let replay = run.run(&page_options(), None).await?;
            check!(eq; (replay.rows, replay.errors), (0, 0));
            check!(eq; run.coaches()?, coaches);
            check!(eq; run.schools()?, schools);
            check!(eq; replay.disposition, crate::CollectionDisposition::Complete);
            check!(eq;
                run.store.journal_payloads(super::super::collect::JOURNAL)?,
                completions
            );
            Ok(())
        })
}

fn assert_postal_claims(school: &CanonicalSchool) -> Result<(), Box<dyn std::error::Error>> {
    let digest = content_digest(RETAINED);
    check!(eq; school.postal_addresses.len(), 3);
    let kinds: Vec<_> = school
        .postal_addresses
        .iter()
        .map(|claim| {
            let note = claim
                .evidence()
                .note
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("postal evidence"))?;
            let note: serde_json::Value = serde_json::from_str(note)?;
            let kind = note
                .get("address_kind")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("address kind"))?;
            let street = if kind == "Shipping" {
                "6281 Arno Rd."
            } else {
                "6281 Arno Rd"
            };
            check!(eq;
                claim.address().line1().map(|line| line.as_str()),
                Some(street)
            );
            check!(eq; claim.address().zip().map(|zip| zip.code()), Some("37064"));
            check!(eq; claim.address().city().map(|city| city.as_str()), Some("Franklin"));
            check!(eq; claim.address().state(), Some(census_domain::UsJurisdiction::Tennessee));
            check!(eq; claim.owner().id, "157");
            check!(eq; claim.evidence().observed_on, CAPTURED);
            check!(eq; claim.capture_sha256(), digest);
            check!(eq;
                claim.evidence().source.url.as_deref(),
                Some("https://portal.tssaa.org/common/directory/?id=157")
            );
            Ok(kind.to_string())
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    check!(eq; kinds, vec!["Mailing", "Physical", "Shipping"]);
    Ok(())
}

fn assert_completion(
    completions: &[serde_json::Value],
    school: &CanonicalSchool,
) -> anyhow::Result<()> {
    check!(eq; completions.len(), 1);
    let receipt = completions
        .first()
        .ok_or_else(|| anyhow::anyhow!("source projection completion"))?;
    check!(eq; receipt["complete"], true);
    check!(eq; receipt["school_id"], "157");
    check!(eq; receipt["school"], school.name);
    check!(eq; receipt["capture_url"], format!("{DIRECTORY_URL}?id=157"));
    check!(eq; receipt["capture_sha256"], content_digest(RETAINED));
    check!(eq; receipt["captured_at"], CAPTURED);
    check!(eq; receipt["school_year"], 2026);
    Ok(())
}
