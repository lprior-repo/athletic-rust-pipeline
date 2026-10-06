use super::FixtureRun;
use crate::ohsaa::{collect, Options};
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
    std::fs::create_dir_all(run.store.out_dir())?;
    std::fs::write(
        run.store.out_dir().join("schools.jsonl"),
        serde_json::to_string(school)? + "\n",
    )?;
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
            check!(eq; run.counts()?, (1, 5, 1));
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
                export_school(&run, &school(identities))?;
                let ctx = run.context("2026-10-02", None)?;
                let report = collect(&ctx, &Options::default()).await?;
                check!(eq; (report.rows, report.errors, report.with_email), (0, 1, 0));
                check!(eq; report.requests, 0);
                check!(eq; run.counts()?, (0, 0, 0));
                check!(run.store.journal_keys("ohsaa_schools")?.is_empty());
                check!(report
                    .notes
                    .iter()
                    .any(|note| note.contains("unfinished acquisition obligation")));
            }
            Ok(())
        })
}
