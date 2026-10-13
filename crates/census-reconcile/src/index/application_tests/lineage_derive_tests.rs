use super::super::{apply, derive, IndexReport};
use super::{resolved_transfer, season, TestResult};
use census_domain::model::{
    AppliedAthleteIdentity, AppliedIdentityKind, CanonicalAthlete, CollectionSnapshot, ReviewCase,
    ReviewState, ReviewVerdictRecord, StoreIdentityError, ATHLETE_IDENTITY_FAMILY,
};
use census_report::report::{ReportError, ReportResult};
use census_store::{
    Application, Receipt, Store, StoreError, StoreLineageError, StoreResult, Table, TableWalk,
};

type LineageResult<T> = Result<T, Box<dyn std::error::Error>>;

const LINEAGE_PHASE: &str = "export-lineage-v1";
const LINEAGE_KEY: &str = "store-identity";
const BASELINE_OPERATION: &str = "lineage-existing-derived";
const BASELINE_DIGEST: &str = "lineage-baseline";
const APPLICATION_DATE: &str = "2026-09-26";

fn pending_identity_fixture(store: &Store) -> LineageResult<(ReviewCase, ReviewCase)> {
    let mut stage = store.stage_derived()?;
    for table in Table::ALL {
        if table.generation_partitioned() {
            check!(eq; stage.carry_forward(table)?, 0);
        }
    }
    stage.replace_many(
        Table::Snapshots,
        &[CollectionSnapshot::new("lineage-baseline", "2026-09-25")],
    )?;
    let publication = stage.publish(BASELINE_OPERATION, BASELINE_DIGEST)?;
    check!(eq; publication.generation, 1);
    match publication.application {
        Application::Written(receipt) => {
            check!(eq; receipt.operation.as_str(), BASELINE_OPERATION);
            check!(eq; receipt.digest.as_str(), BASELINE_DIGEST);
            check!(eq; receipt.appended, 0);
        }
        observed => {
            return Err(format!("baseline publication was not first: {observed:?}").into());
        }
    }
    let bound = resolved_transfer(store)?;
    let generic = ReviewCase::pending(
        ATHLETE_IDENTITY_FAMILY,
        &bound.subject_id,
        &bound.subject,
        "Pending generic advice must survive a lineage refusal",
    );
    check!(ne; generic.id, bound.id);
    store.replace(Table::ReviewCases, &generic)?;
    check!(eq; bound.state, ReviewState::Resolved);
    check!(eq; generic.state, ReviewState::Pending);
    check!(eq; bound.member_ids.len(), 2);
    check!(eq; generic.member_ids.len(), 0);
    check!(eq; store.walk_table(Table::AthleteIdentityDecisions)?.rows, 0);
    check!(eq; store.receipt_count()?, 1);
    check!(eq; store.receipt(&supersession_operation(&generic))?, None);
    Ok((bound, generic))
}

fn supersession_operation(generic: &ReviewCase) -> String {
    format!("identity-generic-supersession:{}", generic.id)
}

#[derive(Debug, PartialEq)]
struct RefusalWitness {
    sequence: u64,
    evidence: u64,
    derived: u64,
    tables_digest: String,
    table_walks: Vec<(&'static str, TableWalk)>,
    athletes: Vec<CanonicalAthlete>,
    cases: Vec<ReviewCase>,
    verdicts: Vec<ReviewVerdictRecord>,
    decisions: Vec<AppliedAthleteIdentity>,
    snapshots: Vec<CollectionSnapshot>,
    receipts: Vec<Receipt>,
    lineage_keys: Vec<String>,
    lineage_payload: Option<String>,
}

impl RefusalWitness {
    fn capture(store: &Store) -> LineageResult<Self> {
        let view = store.snapshot();
        let table_walks = Table::ALL
            .into_iter()
            .map(|table| Ok((table.file(), store.walk_table(table)?)))
            .collect::<StoreResult<Vec<_>>>()?;
        let mut receipts = Vec::new();
        let visited = store.for_each_receipt(8, |receipt| {
            receipts.push(receipt);
            Ok(())
        })?;
        check!(eq; visited, receipts.len());
        check!(eq; store.receipt_count()?, u64::try_from(receipts.len())?);
        let mut lineage_keys: Vec<_> = store.journal_keys(LINEAGE_PHASE)?.into_iter().collect();
        lineage_keys.sort_unstable();
        let witness = Self {
            sequence: view.sequence(),
            evidence: view.evidence_generation(),
            derived: view.derived_generation(),
            tables_digest: view.tables_digest(&Table::ALL)?,
            table_walks,
            athletes: view.scan(Table::Athletes)?,
            cases: view.scan(Table::ReviewCases)?,
            verdicts: view.scan(Table::IdentityVerdicts)?,
            decisions: view.scan(Table::AthleteIdentityDecisions)?,
            snapshots: view.scan(Table::Snapshots)?,
            receipts,
            lineage_keys,
            lineage_payload: view
                .journal_payload(LINEAGE_PHASE, LINEAGE_KEY)?
                .map(|payload| payload.to_string()),
        };
        check!(eq; store.snapshot().sequence(), witness.sequence);
        Ok(witness)
    }
}

#[derive(Debug, Clone, Copy)]
enum LineageRefusal {
    InvalidIdentity,
    MalformedEnvelope,
}

fn require_lineage_refusal(
    result: ReportResult<IndexReport>,
    expected: LineageRefusal,
) -> TestResult {
    match (result, expected) {
        (
            Err(ReportError::Store(StoreError::StoreIdentity(StoreIdentityError::InvalidFormat))),
            LineageRefusal::InvalidIdentity,
        ) => Ok(()),
        (
            Err(ReportError::Store(StoreError::StoreLineage(StoreLineageError::MalformedEnvelope))),
            LineageRefusal::MalformedEnvelope,
        ) => Ok(()),
        (observed, expected) => {
            Err(format!("derive returned {observed:?}; required {expected:?}").into())
        }
    }
}

fn lineage_refusal_preserves_pending_application(
    payload: Option<&str>,
    expected: LineageRefusal,
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), 8_388_608)?;
    let (bound, generic) = pending_identity_fixture(&store)?;
    store.journal_done(LINEAGE_PHASE, LINEAGE_KEY, &payload)?;
    let before = RefusalWitness::capture(&store)?;
    check!(eq; before.derived, 1);
    check!(eq; before.athletes.len(), 2);
    check!(eq; before.cases.len(), 2);
    check!(eq; before.verdicts.len(), 1);
    check!(eq; before.decisions.len(), 0);
    check!(eq; before.snapshots.len(), 1);
    check!(eq; before.receipts.len(), 1);
    check!(eq; before.lineage_keys, vec![LINEAGE_KEY.to_owned()]);
    check!(eq; before.cases.iter().find(|case| case.id == bound.id), Some(&bound));
    check!(eq; before.cases.iter().find(|case| case.id == generic.id), Some(&generic));

    require_lineage_refusal(
        derive(&store, "lineage-refused", APPLICATION_DATE, season()?),
        expected,
    )?;
    let after = RefusalWitness::capture(&store)?;
    check!(eq; after.evidence, before.evidence);
    check!(eq; after.derived, before.derived);
    check!(eq; after, before);
    check!(eq; store.receipt(&supersession_operation(&generic))?, None);
    check!(eq; store.journal_contains("identity_generic_supersession_v1", &generic.id)?, false);
    Ok(())
}

#[test]
fn derive_preserves_pending_identity_application_when_lineage_has_invalid_hex() -> TestResult {
    lineage_refusal_preserves_pending_application(
        Some("gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg"),
        LineageRefusal::InvalidIdentity,
    )
}

#[test]
fn derive_preserves_pending_identity_application_when_lineage_has_wrong_length() -> TestResult {
    lineage_refusal_preserves_pending_application(
        Some("0123456789abcdef"),
        LineageRefusal::InvalidIdentity,
    )
}

#[test]
fn derive_preserves_pending_identity_application_when_lineage_payload_is_null() -> TestResult {
    lineage_refusal_preserves_pending_application(None, LineageRefusal::MalformedEnvelope)
}

#[test]
fn pending_transfer_is_applied_and_supersedes_generic_case_when_application_runs() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), 8_388_608)?;
    let (bound, generic) = pending_identity_fixture(&store)?;
    let before = RefusalWitness::capture(&store)?;
    check!(eq; apply::apply_decisions(&store, APPLICATION_DATE)?, 1);
    let decisions = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    check!(eq; decisions.len(), 1);
    let decision = decisions
        .first()
        .ok_or("missing pending transfer application")?;
    let canonical = bound
        .member_ids
        .iter()
        .min()
        .ok_or("missing transfer members")?;
    check!(eq; decision.kind, AppliedIdentityKind::SamePerson);
    check!(eq; decision.case_id.as_deref(), Some(bound.id.as_str()));
    check!(eq; decision.canonical_id.as_ref().map(|id| id.as_str()), Some(canonical.as_str()));
    check!(eq; decision.observed_at.as_str(), APPLICATION_DATE);
    let mut members: Vec<_> = decision
        .members
        .iter()
        .map(|member| &member.subject)
        .collect();
    members.sort_unstable();
    let mut expected_members: Vec<_> = bound.member_ids.iter().collect();
    expected_members.sort_unstable();
    check!(eq; members, expected_members);
    let mut superseded = generic.clone();
    superseded.state = ReviewState::Superseded;
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    check!(eq; cases.iter().find(|case| case.id == generic.id), Some(&superseded));
    check!(eq; cases.iter().find(|case| case.id == bound.id), Some(&bound));
    check!(eq; store.evidence_generation(), before.evidence.checked_add(2).ok_or("generation overflow")?);
    check!(eq; store.derived_generation(), before.derived);
    check!(eq; store.receipt_count()?, 2);
    let receipt = store
        .receipt(&supersession_operation(&generic))?
        .ok_or("missing generic supersession receipt")?;
    check!(eq; receipt.operation, supersession_operation(&generic));
    check!(eq; receipt.digest, decision.id);
    check!(eq; receipt.appended, 0);
    check!(eq; store.receipt(BASELINE_OPERATION)?, before.receipts.first().cloned());
    check!(eq; store.journal_contains("identity_generic_supersession_v1", &generic.id)?, true);
    let projection = store.athlete_identity_projection()?;
    for member in &bound.member_ids {
        check!(eq; projection.canonical_id(member.as_str()), canonical.as_str());
        check!(eq; projection.status(member.as_str())?, census_domain::model::IdentityStatus::Verified);
    }
    Ok(())
}
