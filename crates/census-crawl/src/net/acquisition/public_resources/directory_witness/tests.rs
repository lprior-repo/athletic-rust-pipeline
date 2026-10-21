use super::*;
use census_domain::model::*;
use census_domain::UsJurisdiction;
use fixture::fixture;

mod fixture;
mod refusals;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const URL: &str = "https://mi.milesplit.com/teams";
const TEAM: &str = "12439";
const ROSTER_URL: &str = "https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster";
const DIRECTORY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/participation_evidence/provider_school/fixtures/mi-team-index.html"
));

#[derive(Debug, PartialEq, Eq)]
struct StoreWitness {
    sequence: u64,
    receipts: u64,
    keys: Vec<String>,
}

impl StoreWitness {
    fn read(store: &census_store::Store) -> TestResult<Self> {
        let mut keys: Vec<_> = store
            .journal_keys(CaptureProtocolVersion::V2.phase())?
            .into_iter()
            .collect();
        keys.sort();
        Ok(Self {
            sequence: store.snapshot().sequence(),
            receipts: store.receipt_count()?,
            keys,
        })
    }
}

#[test]
fn genuine_directory_body_with_controlled_transport_yields_exact_literal_witness() -> TestResult {
    let f = fixture(DIRECTORY, 200)?;
    let before = StoreWitness::read(&f.store)?;
    let evidence = f.evidence()?;
    let checked = assess_directory_roster_child(&evidence, &f.captured, TEAM)?;
    check!(std::ptr::eq(checked.evidence(), &evidence));
    check!(eq; checked.team_id(), TEAM);
    check!(eq; checked.roster_url(), ROSTER_URL);
    check!(eq; checked.row(), DomainLocator::new(241, 91718, 271));
    check!(eq; checked.anchor(), DomainLocator::new(241, 91756, 137));
    check!(eq; checked.name(), DomainLocator::new(241, 91735, 176));
    check!(eq; checked.city(), DomainLocator::new(241, 91924, 51));
    check!(eq; checked.semantic_contract(),
        PublicDirectorySemanticContractV1::MileSplitActiveDirectoryRosterV1);
    check!(checked.parsed_teams() > 0);
    check!(eq; checked.kernel_digest().as_str().len(), 64);
    let row = DIRECTORY.get(91718..91989).ok_or("genuine literal row")?;
    check!(eq; checked.row_digest().as_str(), crate::net::cache::content_digest(row));
    check!(eq; f.captured.capture().content_digest,
        "39606bf8e07f0170138bb586ae8a762903bb820b515520e89e34745f6c732351");
    check!(eq; before, StoreWitness::read(&f.store)?);
    Ok(())
}

#[test]
fn cold_reopen_reassesses_original_parent_and_preserves_exact_bytes_without_writes() -> TestResult {
    let f = fixture(DIRECTORY, 200)?;
    let metadata = f
        .captured
        .original_metadata
        .as_deref()
        .ok_or("normalized metadata")?
        .to_vec();
    let reference = f.reference.clone();
    let f = f.reopen()?;
    let before = StoreWitness::read(&f.store)?;
    let evidence = f.evidence()?;
    let checked = assess_directory_roster_child(&evidence, &f.captured, TEAM)?;
    check!(eq; evidence.retained().reference(), &reference);
    check!(eq; evidence.retained().body_reference(), &reference);
    check!(eq; evidence.retained().prepared().metadata_bytes(), metadata);
    check!(eq; f.captured.capture().body, DIRECTORY);
    check!(f.captured.capture().from_cache);
    check!(eq; checked.roster_url(), ROSTER_URL);
    check!(eq; checked.row(), DomainLocator::new(241, 91718, 271));
    check!(eq; before, StoreWitness::read(&f.store)?);
    Ok(())
}

#[test]
fn native_id_validation_and_absent_child_never_rewrite_retained_directory() -> TestResult {
    let f = fixture(DIRECTORY, 200)?;
    let evidence = f.evidence()?;
    let before = StoreWitness::read(&f.store)?;
    for native in ["0", "012439", "12439/roster", "999999999", "", " 12439"] {
        check!(
            assess_directory_roster_child(&evidence, &f.captured, native).is_err(),
            "accepted invalid or absent native team {native}"
        );
    }
    check!(eq; before, StoreWitness::read(&f.store)?);
    Ok(())
}
