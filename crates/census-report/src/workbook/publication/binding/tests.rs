use super::*;
use crate::export::DatasetLineage;
use crate::report::Scope;
use crate::workbook::publication::{pointer, recover_publication, RecoveryOutcome};
use census_domain::model::SchoolYear;
use census_store::publish::{Intent, Request, State};
use census_store::Table;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const PREDECESSOR_POLICY21_RECIPE: &str =
    "4ddc63f6106c58c5fb8737e5ceff572df47682ae67e42f1572f0e82540b6c8c8";
const PREDECESSOR_POLICY22_RECIPE: &str =
    "9d20b03207f81117a1d1bd37a989dd3bf18577974783aea9347ca7087593373a";

fn options() -> TestResult<Options> {
    Ok(Options {
        address_corpus: None,
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("fixture school year")?,
        jurisdictions: Vec::new(),
    })
}

fn lineage(policy_revision: u32) -> DatasetLineage {
    DatasetLineage {
        store_root: String::new(),
        generated_on: String::new(),
        store_identity: String::new(),
        export_job: None,
        input_generation: String::new(),
        input_digest: String::new(),
        source_digest: String::new(),
        snapshot_sequence: 0,
        schema_revision: 3,
        policy_revision,
        run: None,
        cohort: None,
    }
}

#[test]
fn supported_artifact_six_preserves_distinct_predecessor_policy_recipes() -> TestResult {
    let options = options()?;
    for (policy, expected) in [
        (21, PREDECESSOR_POLICY21_RECIPE),
        (22, PREDECESSOR_POLICY22_RECIPE),
    ] {
        let lineage = lineage(policy);
        check!(eq; recipe_for_artifact(&lineage, &options, 6)?.as_str(), expected);
        check!(eq; recipe(&lineage, &options)?.as_str(), expected);
    }
    check!(ne; PREDECESSOR_POLICY21_RECIPE, PREDECESSOR_POLICY22_RECIPE);
    Ok(())
}

fn predecessor_intent(store: &Store, root: &Path, desired: &Digest) -> TestResult<Intent> {
    let dataset = ExportDataset::load(store)?;
    check!(eq; dataset.lineage.schema_revision, 3);
    check!(eq; dataset.lineage.policy_revision, 22);
    let mut input = input(&dataset, store, &options()?)?;
    input.recipe = Digest::parse(PREDECESSOR_POLICY22_RECIPE)?;
    let namespace = namespace(root)?;
    let identity = digest(&("pinned-c1a-recipe-fence", &namespace))?;
    let request = store.publication_request(Request {
        namespace,
        lease_epoch: store.publication_lease(&identity)?,
        identity,
        expected: pointer::read(root)?,
        input,
    })?;
    let manifest = run_manifest(store)?;
    let intent = {
        let fence = store.publication_fence(&request.request, &manifest)?;
        fence.begin(fence.intent(), desired)?
    };
    Ok(store.publication_mark(
        &intent,
        State::Published {
            receipt: desired.clone(),
        },
    )?)
}

#[test]
fn pinned_predecessor_recipe_fence_survives_reopen_and_recovery() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store_root = directory.path().join("store");
    let store = Store::open(&store_root)?;
    let workbook = crate::workbook::build(&store, &options()?)?;
    let root = store.out_dir().join("publication");
    let desired = match pointer::read(&root)? {
        census_store::publish::Root::Bundle(value) => value,
        census_store::publish::Root::Absent => return Err("published bundle absent".into()),
    };
    let original = std::fs::read(&workbook)?;
    let retained = predecessor_intent(&store, &root, &desired)?;
    drop(store);
    let reopened = Store::open(&store_root)?;
    let sequence = reopened.snapshot().sequence();
    let physical = reopened.snapshot().tables_digest(&Table::ALL)?;
    for _attempt in 0..2 {
        pointer::validate_binding(&root, &desired, &retained.request.input)?;
        check!(eq; recover_publication(&reopened, &root)?, RecoveryOutcome::HistoricalSuccess {
            bundle: desired.clone()
        });
        let current = reopened
            .publication_read(&retained.request.identity)?
            .ok_or("retained publication intent absent")?;
        check!(eq; current, retained);
        check!(eq; std::fs::read(&workbook)?, original);
        check!(eq; reopened.snapshot().sequence(), sequence);
        check!(eq; reopened.snapshot().tables_digest(&Table::ALL)?, physical);
    }
    Ok(())
}
