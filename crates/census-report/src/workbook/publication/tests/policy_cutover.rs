use super::*;
use census_store::publish::{Digest, Intent, Root, State};
use std::collections::BTreeMap;

mod historical;

fn generation_bytes(directory: &Path) -> TestResult<BTreeMap<String, Vec<u8>>> {
    std::fs::read_dir(directory)?
        .map(|entry| {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "fixture filename")?;
            Ok((name, std::fs::read(entry.path())?))
        })
        .collect()
}

fn cutover(grad_year: Option<i16>, prior_policy: u32) -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store_path = temporary.path().join("store");
    let store = Store::open(&store_path)?;
    let root = store.out_dir().join("publication");
    let options = Options {
        grad_year,
        out: Some(root.clone()),
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("fixture season")?,
        jurisdictions: Vec::new(),
    };
    let old = historical::install(&store, &root, &options, prior_policy)?;
    let old_directory = old.workbook.parent().ok_or("historical directory")?;
    let retained = generation_bytes(old_directory)?;
    let lease = store.get_lease_record(old.intent.request.identity.as_str())?;
    historical_input_is_read_only(&old)?;
    let current = ExportDataset::load(&store)?;
    check!(eq; current.lineage.input_digest, old.lineage.input_digest);
    check!(eq; current.lineage.source_digest, old.lineage.source_digest);
    check!(ne; current.lineage.input_generation, old.lineage.input_generation);
    let input = binding::input(&current, &store, &options)?;
    check!(ne; input.recipe, old.intent.request.input.recipe);
    check!(ne; input.token, old.intent.request.input.token);
    let identity = binding::digest(&(
        &old.intent.request.namespace,
        &current.lineage.input_generation,
        &input.recipe,
    ))?;
    check!(ne; identity, old.intent.request.identity);
    check!(eq; store.publication_read(&identity)?, None);
    let censuses = crate::workbook::Censuses::of(&current, &store.out_dir(), &[])?;
    let published = crate::workbook::build_from(&current, &store, &options, &censuses)?;
    check!(ne; published, old.workbook);
    check!(eq; current_workbook(&root)?, published);
    let directory = published.parent().ok_or("new generation directory")?;
    let manifest = manifest::verify_directory(directory)?;
    let receipt = Digest::parse(&manifest.generation_digest)?;
    let new_intent = store
        .publication_read(&identity)?
        .ok_or("new published request")?;
    check!(eq; new_intent.state, State::Published { receipt: receipt.clone() });
    check!(eq; new_intent.request.expected, Root::Bundle(old.receipt.clone()));
    check!(eq; store.publication_read(&old.intent.request.identity)?, Some(old.intent.clone()));
    check!(eq; store.get_lease_record(old.intent.request.identity.as_str())?, lease);
    check!(eq; generation_bytes(old_directory)?, retained);
    check!(eq; crate::workbook::build_from(&current, &store, &options, &censuses)?, published);
    drop(current);
    drop(store);
    let reopened = Store::open(&store_path)?;
    check!(eq; reopened.publication_read(&old.intent.request.identity)?, Some(old.intent.clone()));
    check!(eq; reopened.publication_read(&identity)?, Some(new_intent));
    check!(eq; generation_bytes(old_directory)?, retained);
    check!(eq; current_workbook(&root)?, published);
    check!(eq; verify_published(&published)?.generation_digest, receipt.as_str());
    historical_input_is_read_only(&old)?;
    check!(eq; generation_bytes(old_directory)?, retained);
    Ok(())
}

#[test]
fn current_policy_cannot_reuse_historical_published_receipts() -> TestResult {
    [Some(2027), Some(2028), None]
        .into_iter()
        .try_for_each(|year| {
            [20, 19, 18, 17, 16]
                .into_iter()
                .try_for_each(|policy| cutover(year, policy))
        })
}

fn historical_input_is_read_only(old: &historical::Historical) -> TestResult {
    let directory = old.workbook.parent().ok_or("historical directory")?;
    manifest::verify_directory(directory)?;
    if matches!(old.lineage.policy_revision, 18..=20) {
        let dataset = ExportDataset::reopen_frozen(&directory.join("frozen-input.json"))?;
        check!(eq; dataset.lineage, old.lineage);
        let semantics = if old.lineage.policy_revision == 18 {
            census_domain::model::EventSemantics::Legacy18
        } else if old.lineage.policy_revision == 19 {
            census_domain::model::EventSemantics::FlatEvents19
        } else {
            census_domain::model::EventSemantics::CombinedEvents20
        };
        check!(eq; dataset.semantic_policy()?, semantics);
        check!(matches!(
            dataset.ensure_current_policy(),
            Err(crate::report::ReportError::Invariant { ref detail })
                if detail == "historical recruiting policy is read-only and cannot capture or publish"
        ));
    } else {
        check!(matches!(
            verify_published(&old.workbook),
            Err(crate::report::ReportError::Invariant { ref detail })
                if detail == "unsupported or malformed frozen export lineage"
        ));
    }
    Ok(())
}
