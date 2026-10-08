use super::FixtureRun;
use crate::ohsaa::{collect, Options, HOST, SEARCH_PATH};
use census_domain::model::{CanonicalSchool, SourceIdentity, SourceNamespace};
use census_domain::UsJurisdiction;

fn school(identities: Vec<SourceIdentity>) -> CanonicalSchool {
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Ohio,
        "DUBLIN COFFMAN",
        "dublin coffman",
        Some("Dublin"),
    );
    school.association = Some("ohsaa".to_string());
    school.source_identities = identities;
    school
}

fn export_school(run: &FixtureRun, school: &CanonicalSchool) -> anyhow::Result<()> {
    run.store.append(census_store::Table::Schools, school)?;
    Ok(())
}

#[test]
fn fallback_selects_ohsaa_owner_instead_of_first_unrelated_association(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(Some(super::super::fixture_ad_dublin()))?;
            let school = school(vec![
                SourceIdentity::new(SourceNamespace::association_school("wiaa"), "991"),
                SourceIdentity::new(SourceNamespace::association_school("ohsaa"), "474"),
            ]);
            export_school(&run, &school)?;
            let ctx = run.context("2026-10-02", None)?;
            let report = collect(&ctx, &Options::default()).await?;
            check!(eq; (report.rows, report.errors, report.with_email), (1, 0, 5));
            let receipt = run.outcome()?;
            check!(eq; receipt["ohsaa_id"], "474");
            check!(eq;
                receipt["sports_capture"]["capture_url"],
                super::dublin().sports_url()
            );
            Ok(())
        })
}

#[test]
fn absent_or_empty_ohsaa_owner_is_explicit_unfinished_work_without_guessed_requests(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for identities in [
                vec![SourceIdentity::new(
                    SourceNamespace::association_school("wiaa"),
                    "474",
                )],
                vec![SourceIdentity::new(
                    SourceNamespace::association_school("ohsaa"),
                    "",
                )],
            ] {
                let run = FixtureRun::new(Some(super::super::fixture_ad_dublin()))?;
                let school = school(identities);
                export_school(&run, &school)?;
                let ctx = run.context("2026-10-02", None)?;
                let report = collect(&ctx, &Options::default()).await?;
                check!(eq; (report.rows, report.errors, report.with_email), (0, 1, 0));
                check!(eq; report.requests, 0);
                check!(eq; report.from_cache, 0);
                check!(eq; run.store.scan::<CanonicalSchool>(census_store::Table::Schools)?, vec![school.clone()]);
                check!(eq; run.store.walk_table(census_store::Table::SourceObservations)?.rows, 0);
                check!(eq; run.store.scan::<census_domain::model::CanonicalCoach>(census_store::Table::Coaches)?, vec![]);
                check!(run.store.journal_keys("ohsaa_schools")?.is_empty());
                check!(run.store.journal_keys(super::OUTCOMES)?.is_empty());
                check!(run.store.journal_keys(super::CAPTURES)?.is_empty());
                check!(eq; report.disposition, crate::CollectionDisposition::Partial);
                check!(eq;
                    report.unfinished.into_iter().collect::<std::collections::BTreeSet<_>>(),
                    [
                        format!("ohsaa:school:{}", school.id),
                        format!("{HOST}{SEARCH_PATH}"),
                    ].into_iter().collect::<std::collections::BTreeSet<_>>()
                );
            }
            Ok(())
        })
}
