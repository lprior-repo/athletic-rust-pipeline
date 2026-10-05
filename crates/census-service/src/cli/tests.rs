use super::{resolve_restriction, resolve_states};
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn all_states_selects_the_census_run_scope() -> TestResult {
    let states = resolve_states(true, &[])?;
    check!(eq; states, UsJurisdiction::CENSUS_SCOPE);
    check!(!states.contains(&UsJurisdiction::Alaska));
    Ok(())
}

#[test]
fn no_flag_defaults_to_wisconsin() -> TestResult {
    let states = resolve_states(false, &[])?;
    check!(eq; states, vec![UsJurisdiction::Wisconsin]);
    Ok(())
}

#[test]
fn an_explicit_list_is_kept_in_caller_order() -> TestResult {
    let asked = vec![UsJurisdiction::Ohio, UsJurisdiction::Iowa];
    let states = resolve_states(false, &asked)?;
    check!(eq; states, asked);
    Ok(())
}

#[test]
fn combining_the_two_flags_is_refused() -> TestResult {
    let error = match resolve_states(true, &[UsJurisdiction::Ohio]) {
        Err(error) => error,
        Ok(_) => return Err("both flags accepted".into()),
    };
    check!(error.to_string().contains("--all-states"));
    Ok(())
}

#[test]
fn a_restriction_with_no_flag_is_empty_not_wisconsin() -> TestResult {
    let states = resolve_restriction(false, &[])?;
    check!(states.is_empty());
    let all = resolve_restriction(true, &[])?;
    check!(eq; all, UsJurisdiction::CENSUS_SCOPE);
    let explicit = resolve_restriction(false, &[UsJurisdiction::Ohio])?;
    check!(eq; explicit, vec![UsJurisdiction::Ohio]);
    check!(resolve_restriction(true, &[UsJurisdiction::Ohio]).is_err());
    Ok(())
}

use super::verify::{run_verify, VerifyArgs};
use census_domain::model::{CanonicalSchool, Evidence, SourceRef};
use census_store::{Store, Table};

#[test]
fn verification_uses_the_frozen_generation_after_the_store_changes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let workbook = census_report::workbook::build(&store, &Default::default())?;
    let (mut school, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Later School", "later school");
    school.evidence.push(Evidence::parsed(
        SourceRef::new(
            "wiaa_results",
            Some("https://example.test/results".to_string()),
        ),
        "2026-06-01",
    ));
    store.append(Table::Schools, &school)?;
    run_verify(
        store.root(),
        &VerifyArgs {
            workbook: Some(workbook),
        },
    )?;
    Ok(())
}

#[test]
fn verification_refuses_a_changed_artifact_in_the_published_bundle() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let workbook = census_report::workbook::build(&store, &Default::default())?;
    std::fs::write(&workbook, "damaged workbook")?;
    let error = match run_verify(
        store.root(),
        &VerifyArgs {
            workbook: Some(workbook),
        },
    ) {
        Err(error) => error,
        Ok(_) => return Err("corrupted workbook accepted".into()),
    };
    check!(format!("{error:#}").contains("generation artifact mismatch: workbook.xlsx"));
    Ok(())
}

#[test]
fn a_cli_built_fetcher_refuses_an_origin_another_run_holds() -> TestResult {
    use std::io::Write;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let cli =
        <super::Cli as clap::Parser>::parse_from(["census-service", "provider", "home_campus"]);
    let host = format!("origin-locks-{}.test", std::process::id());
    let origin_url = format!("https://{host}");
    let root = std::path::Path::new(census_service::census::DEFAULT_ORIGIN_LOCK_ROOT);
    std::fs::create_dir_all(root)?;
    let lock_path = root.join(census_crawl::net::origin_lock_file_name(&origin_url));
    let mut holder = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;
    holder.try_lock()?;
    holder.write_all(b"{\"pid\":424242}")?;
    let fetcher = super::build_fetcher_authorizing(&cli, &store, vec![host.clone()])?;
    let outcome = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            fetcher
                .get(&format!("{origin_url}/teams"), &Default::default())
                .await
        });
    drop(holder);
    std::fs::remove_file(&lock_path)?;
    if let Some(parent) = root.parent() {
        let _ = std::fs::remove_dir(root);
        let _ = std::fs::remove_dir(parent);
    }
    match outcome {
        Err(census_crawl::net::FetchError::OriginHeld {
            origin,
            holder: named,
        }) => {
            check!(eq; origin, origin_url);
            check!(named.contains("424242"));
        }
        other => return Err(format!("unexpected outcome {other:?}").into()),
    }
    Ok(())
}
