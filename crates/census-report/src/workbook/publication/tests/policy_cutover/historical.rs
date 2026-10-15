use super::*;
use crate::export::DatasetLineage;
use crate::workbook::publication::manifest::historical_fixture;
use census_store::publish::Request;

pub(super) struct Historical {
    pub(super) workbook: PathBuf,
    pub(super) lineage: DatasetLineage,
    pub(super) intent: Intent,
    pub(super) receipt: Digest,
}

pub(super) fn install(
    store: &Store,
    root: &Path,
    options: &Options,
    policy: u32,
) -> TestResult<Historical> {
    let mut dataset = ExportDataset::load(store)?;
    historical_fixture::rebind(&mut dataset, policy)?;
    let namespace = binding::namespace(root)?;
    let stage = root.join(format!("synthetic-policy{policy}-integrity-fixture"));
    std::fs::create_dir_all(root.join("generations"))?;
    std::fs::create_dir(&stage)?;
    write_inventory(&stage, &dataset, options)?;
    super::super::super::catalog_tests::transition::reconstructed::write_revision_two_manifest(
        &stage,
        &dataset.lineage,
        options,
    )?;
    let manifest = manifest::verify_directory(&stage)?;
    let receipt = Digest::parse(&manifest.generation_digest)?;
    let installed = root.join("generations").join(receipt.as_str());
    std::fs::rename(&stage, &installed)?;
    manifest::verify_directory(&installed)?;
    let input = historical_fixture::input(store, &dataset, options, 2)?;
    let identity =
        binding::digest(&(&namespace, &dataset.lineage.input_generation, &input.recipe))?;
    let lease_epoch = store.publication_lease(&identity)?;
    let request = store.publication_request(Request {
        namespace,
        identity,
        expected: Root::Absent,
        input,
        lease_epoch,
    })?;
    let fence = store.publication_fence(&request.request, &request.request.input.run_manifest)?;
    let intent = fence.begin(fence.intent(), &receipt)?;
    pointer::switch(root, &receipt)?;
    sync_directory(root)?;
    let intent = fence.published(&intent, &receipt)?;
    drop(fence);
    pointer::validate_binding(root, &receipt, &intent.request.input)?;
    Ok(Historical {
        workbook: installed.join("workbook.xlsx"),
        lineage: dataset.lineage.clone(),
        intent,
        receipt,
    })
}

fn write_inventory(directory: &Path, dataset: &ExportDataset, options: &Options) -> TestResult {
    dataset.save_frozen(&directory.join("frozen-input.json"))?;
    let cohort = match options.grad_year {
        Some(year) => format!("co{year}"),
        None => "all".to_string(),
    };
    std::fs::write(
        directory.join(format!("best-results-{cohort}.csv")),
        "Athlete ID\n",
    )?;
    std::fs::write(directory.join(format!("best-results-{cohort}.jsonl")), "")?;
    for (name, content) in [
        ("audit.json", "{}"),
        ("census-all-sources.json", "{}"),
        ("census-core.json", "{}"),
        ("recruiting.csv", "Athlete ID\n"),
        ("school-contacts.csv", "School ID\n"),
        ("contact-research.csv", "School ID\n"),
        ("event-catalog.json", "{}"),
        ("relay-dispositions.json", "{}"),
        ("relays.jsonl", ""),
    ] {
        std::fs::write(directory.join(name), content)?;
    }
    let mut book = rust_xlsxwriter::Workbook::new();
    for name in [
        "Athletes",
        "PRs",
        "Performances_001",
        "Coaches",
        "Schools",
        "Meets",
        "Sources",
        "Coverage",
        "Conflicts",
        "Review",
        "Run Metrics",
        "School Contacts",
        "Contact Research",
    ] {
        book.add_worksheet().set_name(name)?;
    }
    book.save(directory.join("workbook.xlsx"))?;
    Ok(())
}
