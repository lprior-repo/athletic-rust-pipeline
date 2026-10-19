use super::*;
use crate::roster_source_issuance::{RosterEnrollmentReviewV1, RosterReviewActorV1};
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use chrono::DateTime;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn enrolled() -> TestResult<(tempfile::TempDir, Store)> {
    let (directory, store, _writer) = crate::public_identity_tests::support::fixture()?;
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    let review = RosterEnrollmentReviewV1 {
        operation: RosterAuthorityTextV1::parse("controlled-inspection-enrollment")?,
        actor: RosterReviewActorV1::LocalOperator,
        reviewed_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")?.to_utc(),
        rationale: RosterAuthorityTextV1::parse("Controlled root and grant inspection test")?,
    };
    let prepared = store.prepare_roster_source_enrollment_v1(inputs, review)?;
    let bytes = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    store.apply_roster_source_enrollment_v1(prepared, &bytes, &digest)?;
    Ok((directory, store))
}

fn request(store: &Store) -> TestResult<RosterIssuanceInspectionRequestV1> {
    let body = b"controlled transport only, not parsed school authority";
    let capture = serde_json::json!({
        "requested_url":"https://example.test/roster", "final_url":"https://example.test/roster",
        "status":200, "acquired_at":"2026-10-16T02:00:00Z", "body_bytes":body.len(),
        "body_sha256":codec::digest(body), "body_base64":STANDARD.encode(body),
        "provenance":{"request_method":"GET","request_headers":[],"response_header_bytes":[],
            "content_type":"text/html; charset=utf-8","body_encoding":"identity","started_at":"2026-10-16T02:00:00Z"}
    });
    let packet = serde_json::json!({"version":1,"interpretation":"milesplit_directory_roster_school_v1",
        "directory":capture,"roster":capture});
    let key = codec::digest(&serde_json::to_vec(&packet)?);
    let schema = RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1;
    store.journal_done(schema.phase(), &key, &packet)?;
    let held = store
        .snapshot()
        .read_retained_capture_root_v1(schema, &key, &mut AttachmentReplayGrantV1::new())?
        .ok_or("controlled root absent")?;
    Ok(RosterIssuanceInspectionRequestV1 {
        root: held.root().clone(),
        school: SchoolId::mint("sch", &["controlled-request"]),
        program: TeamId::mint("team", &["controlled-request"]),
    })
}

#[test]
fn semantic_access_rejects_recharged_foreign_grant_before_swap_restore() -> TestResult {
    let (_directory, store) = enrolled()?;
    let mut inspection = store
        .inspect_roster_source_issuance_v1(request(&store)?, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let usage = grant.usage();
    let mut foreign = AttachmentReplayGrantV1::new();
    foreign.charge_bytes(usage.encoded_bytes())?;
    foreign.charge_steps(usage.traversal_steps())?;
    let original = std::mem::replace(grant, foreign);
    check!(evidence.retained_root(grant).is_err());
    check!(evidence.physical_projection(grant).is_err());
    *grant = original;
    check!(evidence.retained_root(grant).is_ok());
    check!(evidence.physical_projection(grant)?.is_empty());
    Ok(())
}

#[test]
fn changed_root_and_poisoned_grant_cannot_create_inspection() -> TestResult {
    let (_directory, store) = enrolled()?;
    let mut selected = request(&store)?;
    selected.root.payload.bytes = selected
        .root
        .payload
        .bytes
        .checked_add(1)
        .ok_or("overflow")?;
    check!(store
        .inspect_roster_source_issuance_v1(selected, AttachmentReplayGrantV1::new())
        .is_err());
    let selected = request(&store)?;
    let mut poisoned = AttachmentReplayGrantV1::new();
    poisoned.poison();
    check!(store
        .inspect_roster_source_issuance_v1(selected, poisoned)
        .is_err());
    Ok(())
}

#[test]
fn projection_manifest_commits_all_tables_and_exact_snapshot() -> TestResult {
    let (_directory, store) = enrolled()?;
    let mut inspection = store
        .inspect_roster_source_issuance_v1(request(&store)?, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let first = evidence.physical_projection_manifest(grant)?;
    let second = evidence.physical_projection_manifest(grant)?;
    check!(eq; first, second);
    let value: serde_json::Value = serde_json::from_slice(&first)?;
    check!(eq; value.get("schema"), Some(&serde_json::json!("roster-physical-projection-inventory-v1")));
    check!(eq; value.get("tables"), Some(&serde_json::json!(["schools","teams","conflicts","source_observations"])));
    check!(eq; value.get("snapshot_sequence"), Some(&serde_json::json!(evidence.snapshot_sequence())));
    check!(eq; value.get("store"), Some(&serde_json::to_value(evidence.store_identity())?));
    Ok(())
}

#[test]
fn arbitrary_school_observation_ids_are_in_the_complete_physical_inventory() -> TestResult {
    use census_domain::model::{SourceNamespace, SourceObservation, SourceSchoolObservation};
    let (_directory, store) = enrolled()?;
    let mut observation = SourceSchoolObservation::new(
        SourceNamespace::MilesplitSchool,
        "12439",
        "controlled-row",
        "Controlled source school",
        "2026-10-16",
    );
    observation.id = "arbitrary-hidden-school-subject".into();
    store.append_many(
        crate::Table::SourceObservations,
        &[SourceObservation::School(observation)],
    )?;
    let mut inspection = store
        .inspect_roster_source_issuance_v1(request(&store)?, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let rows = evidence.physical_projection(grant)?;
    check!(rows
        .iter()
        .any(|row| row.table() == crate::Table::SourceObservations
            && row
                .key()
                .windows(b"arbitrary-hidden-school-subject".len())
                .any(|bytes| bytes == b"arbitrary-hidden-school-subject")));
    Ok(())
}

#[test]
fn reviewed_roster_inventory_accepts_school_rows_above_the_former_8_mib_limit() -> TestResult {
    use census_domain::{model::CanonicalSchool, UsJurisdiction};

    let (_directory, store) = enrolled()?;
    let padding = "x".repeat(50 * 1024);
    let schools: Vec<_> = (0..176)
        .map(|index| {
            CanonicalSchool::new(
                UsJurisdiction::Michigan,
                format!("Controlled school {index} {padding}"),
                format!("controlled-school-{index}"),
                None,
            )
            .0
        })
        .collect();
    for batch in schools.chunks(32) {
        store.append_many(crate::Table::Schools, batch)?;
    }

    let mut inspection = store
        .inspect_roster_source_issuance_v1(request(&store)?, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let bytes = evidence
        .physical_projection(grant)?
        .iter()
        .try_fold(0_u64, |sum, row| {
            let row_bytes = row
                .key()
                .len()
                .checked_add(row.bytes().len())
                .ok_or("physical inventory size overflow")?;
            sum.checked_add(
                u64::try_from(row_bytes).map_err(|_| "physical inventory size overflow")?,
            )
            .ok_or("physical inventory size overflow")
        })?;
    check!(bytes > 8 * 1024 * 1024);
    check!(bytes <= super::inventory::MAX_PHYSICAL_INVENTORY_BYTES);

    drop(inspection);
    let mut grant = AttachmentReplayGrantV1::new();
    let current = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut grant)
        .map_err(|error| format!("capture current reviewed roster: {error}"))?;
    check!(eq; current.claims(&mut grant)?.len(), 0);
    Ok(())
}
