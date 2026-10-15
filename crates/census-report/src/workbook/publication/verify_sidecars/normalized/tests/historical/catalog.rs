use super::super::{change, files, population::Population, read, value, TestResult, FAMILIES};
use census_domain::model::{EventSemantics, RecruitingCatalogV1};
use std::path::Path;

const HISTORICAL: [EventSemantics; 2] = [EventSemantics::Legacy18, EventSemantics::FlatEvents19];

fn population(policy: EventSemantics) -> TestResult<Population> {
    let mut given = Population::new()?;
    crate::workbook::publication::manifest::historical_fixture::rebind(
        &mut given.dataset,
        policy.revision(),
    )?;
    check!(eq; given.dataset.semantic_policy()?, policy);
    Ok(given)
}

fn manual(given: &Population, directory: &Path, policy: EventSemantics) -> TestResult<String> {
    files::write(given, directory)?;
    let digest = RecruitingCatalogV1::with_policy(policy).digest()?;
    check!(ne; digest, RecruitingCatalogV1::new().digest()?);
    change(
        directory,
        "comparison_classes.csv",
        "catalog_digest",
        &digest,
    )?;
    Ok(digest)
}

fn bytes(directory: &Path) -> TestResult<Vec<Vec<u8>>> {
    Ok(FAMILIES
        .iter()
        .map(|name| std::fs::read(directory.join(name)))
        .collect::<Result<_, _>>()?)
}

#[test]
fn historical_catalog_readback_accepts_exact_policy18_and19_digests_without_rewriting() -> TestResult
{
    for policy in HISTORICAL {
        let given = population(policy)?;
        let directory = tempfile::tempdir()?;
        let digest = manual(&given, directory.path(), policy)?;
        let retained = bytes(directory.path())?;
        super::super::super::verify(directory.path(), &given.inputs()?, 3)?;
        check!(eq; bytes(directory.path())?, retained);
        check!(eq; read(directory.path(), "comparison_classes.csv")?.1.len(), 1);
        check!(eq; value(directory.path(), "comparison_classes.csv", 0, "event_identity")?, "Track400m");
        check!(eq; value(directory.path(), "comparison_classes.csv", 0, "policy_version")?, policy.revision().to_string());
        check!(eq; value(directory.path(), "comparison_classes.csv", 0, "catalog_digest")?, digest);
    }
    Ok(())
}

#[test]
fn historical_catalog_readback_rejects_current_digest_without_changing_retained_bytes() -> TestResult
{
    let current = RecruitingCatalogV1::new().digest()?;
    for policy in HISTORICAL {
        let given = population(policy)?;
        let directory = tempfile::tempdir()?;
        let historical = manual(&given, directory.path(), policy)?;
        super::super::super::verify(directory.path(), &given.inputs()?, 3)?;
        change(
            directory.path(),
            "comparison_classes.csv",
            "catalog_digest",
            &current,
        )?;
        let retained = bytes(directory.path())?;
        super::super::rejected(
            super::super::super::verify(directory.path(), &given.inputs()?, 3),
            "comparison_classes.csv",
            &historical,
        )?;
        check!(eq; bytes(directory.path())?, retained);
    }
    Ok(())
}

#[test]
fn historical_catalog_writer_preserves_policy18_and19_labels_and_digests() -> TestResult {
    for policy in HISTORICAL {
        let given = population(policy)?;
        let directory = tempfile::tempdir()?;
        let bests = crate::bests::build_with_exclusions(
            &given.dataset,
            &crate::bests::Options {
                scope: given.options.scope,
                grad_year: given.options.grad_year,
                limit: given.options.limit,
                jurisdictions: given.options.jurisdictions.clone(),
            },
        )?;
        let counts = crate::workbook::publication::normalized::write(
            directory.path(),
            &given.dataset,
            &given.options,
            &bests.selections,
            &bests.combined_exclusions,
        )?;
        check!(eq; counts.get("comparison_classes.csv"), Some(&1));
        let digest = RecruitingCatalogV1::with_policy(policy).digest()?;
        check!(ne; digest, RecruitingCatalogV1::new().digest()?);
        check!(eq; value(directory.path(), "comparison_classes.csv", 0, "event_identity")?, "Track400m");
        check!(eq; value(directory.path(), "comparison_classes.csv", 0, "policy_version")?, policy.revision().to_string());
        check!(eq; value(directory.path(), "comparison_classes.csv", 0, "catalog_digest")?, digest);
        super::super::super::verify(directory.path(), &given.inputs()?, 4)?;
    }
    Ok(())
}
