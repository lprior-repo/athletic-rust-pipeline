use super::*;
use census_domain::model::{SchoolId, SourceNamespace, SourceSubject, TeamId};
use census_domain::roster_affiliation_authority::{
    decode_roster_issuance_proposal_v1, RosterAuthorityLimitsV1,
};
use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
const SCOPE: &[u8] = b"controlled scope";
const YEAR: &[u8] = b"controlled year";
const MERGE: &[u8] = b"controlled merge";

fn proposal(digest: &str, size: usize) -> TestResult<LocallyCheckedRosterProposalV1> {
    let owner = SourceSubject::new(SourceNamespace::MilesplitSchool, "10002".into())?;
    let team = SourceSubject::new(SourceNamespace::MilesplitTeam, "10002".into())?;
    let reference = |id| json!({"digest":format!("{id:064x}"),"bytes":64});
    let contract = |raw: &[u8]| json!({"version":1,"revision":1,"digest":codec::digest(raw)});
    let value = json!({
        "version":1,
        "scope":{"provider":"milesplit","owner":owner,"sport":"cross_country","category":"girls",
            "policy":contract(SCOPE),"relationship":"represents_school_program","temporal":"exact_utc_capture_day"},
        "parser":{"artifact":reference(1),"interpretation":"milesplit-scoped-current-roster/v1",
            "schema":reference(2),"configuration":reference(3),"dependencies":reference(4)},
        "capture":{"occurrence":"b".repeat(64),"body":reference(5),"manifest":reference(6),
            "interpretation":reference(7),"acquired_at":"2026-10-08T00:00:00Z","observed_day":"2026-10-08",
            "requested_url":"https://oh.milesplit.com/teams/10002-mason/roster","final_url":{"status":"unknown"},
            "retained_source_admission":reference(8),"verification_grant":reference(9)},
        "binding":{"school":SchoolId::mint("sch", &["controlled-school"]),
            "program":TeamId::mint("team", &["controlled-program"]),"school_source":owner,"program_source":team,
            "sport":"cross_country","category":"girls","school_year":2026,"year_convention":contract(YEAR),
            "projection_merge_contract":contract(MERGE),"school_projection":reference(10),"program_projection":reference(11),
            "provider_packet":reference(12),"physical_projection_inventory":{"digest":digest,"bytes":size},
            "store":"c".repeat(64),"snapshot_revision":1}
    });
    let limits = RosterAuthorityLimitsV1::new(
        64 * 1024,
        census_domain::roster_affiliation_authority::MAX_ROSTER_AUTHORITY_COMPONENT_BYTES,
    )?;
    Ok(decode_roster_issuance_proposal_v1(
        &serde_json::to_vec(&value)?,
        limits,
    )?)
}

fn supplied(bytes: Vec<u8>) -> RosterSourceReviewObjectsV1 {
    RosterSourceReviewObjectsV1 {
        objects: vec![bytes],
        scope_policy: SCOPE.to_vec(),
        year_convention: YEAR.to_vec(),
        projection_merge_contract: MERGE.to_vec(),
    }
}

#[test]
fn exact_declared_large_manifest_passes_supplied_role_but_generic_roles_still_refuse() -> TestResult
{
    let bytes = vec![b'x'; 4 * 1024 * 1024 + 1];
    let digest = codec::digest(&bytes);
    let claim = proposal(&digest, bytes.len())?;
    let mut objects = Builder::new();
    let mut grant = AttachmentReplayGrantV1::new();
    retain_contracts(&mut objects, &mut grant, supplied(bytes.clone()), &claim)?;
    check!(eq; objects.get(&digest).map(Vec::as_slice), Some(bytes.as_slice()));
    let before = objects.len();
    let usage = grant.usage();
    check!(matches!(
        insert(&mut objects, &bytes, &mut grant),
        Err(crate::StoreError::RosterSourceIssuance(
            Error::BudgetExceeded
        ))
    ));
    check!(matches!(
        insert_owned(&mut objects, bytes, &mut grant),
        Err(crate::StoreError::RosterSourceIssuance(
            Error::BudgetExceeded
        ))
    ));
    check!(eq; objects.len(), before);
    check!(eq; grant.usage().encoded_bytes(), usage.encoded_bytes());
    check!(eq; grant.usage().traversal_steps(), usage.traversal_steps());
    Ok(())
}

#[test]
fn supplied_large_manifest_must_match_both_declared_length_and_digest() -> TestResult {
    let bytes = vec![b'x'; 4 * 1024 * 1024 + 1];
    let digest = codec::digest(&bytes);
    let mut wrong = digest.clone();
    let first = if wrong.starts_with('0') { "1" } else { "0" };
    wrong.replace_range(..1, first);
    let claim = proposal(&wrong, bytes.len())?;
    check!(matches!(
        retain_contracts(
            &mut Builder::new(),
            &mut AttachmentReplayGrantV1::new(),
            supplied(bytes.clone()),
            &claim
        ),
        Err(crate::StoreError::RosterSourceIssuance(Error::Corrupt(
            "supplied physical manifest differs"
        )))
    ));
    let claim = proposal(&digest, bytes.len() + 1)?;
    check!(matches!(
        retain_contracts(
            &mut Builder::new(),
            &mut AttachmentReplayGrantV1::new(),
            supplied(bytes),
            &claim
        ),
        Err(crate::StoreError::RosterSourceIssuance(
            Error::BudgetExceeded
        ))
    ));
    Ok(())
}
