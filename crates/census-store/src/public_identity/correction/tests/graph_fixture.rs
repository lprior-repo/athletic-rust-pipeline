use super::*;
use crate::ReviewedIdentityRequestV1;
use crate::{IdentityCommitOutcomeV1, PublicIdentityWriterV1, Store};
use census_domain::model::{
    AttestationQualification, CanonicalAthlete, CanonicalPerformance, EventId, Evidence, Gender,
    GradYear, IdentityAttestation, Mark, MeetId, PerformanceIdentity, PerformanceResult,
    RelayMember, RelayResult, SchoolId, SourceIdentity, SourceNamespace, SourceRef, TeamId,
};
use census_domain::public_identity_history::BindingPartition;
use sha2::{Digest, Sha256};

pub(super) struct Fixture {
    pub directory: tempfile::TempDir,
    pub store: Store,
    pub writer: PublicIdentityWriterV1,
    pub births: Vec<IdentityCommitOutcomeV1>,
    pub members: Vec<CanonicalAthlete>,
    pub case: String,
    pub affected: Vec<String>,
}

pub(super) fn ready() -> TestResult<Fixture> {
    let (directory, store, writer) = fixture()?;
    let mut members = Vec::new();
    for (school, namespace, source) in [
        (
            "graph-correction-a",
            SourceNamespace::MilesplitAthlete,
            "101",
        ),
        ("graph-correction-b", SourceNamespace::TfrrsAthlete, "102"),
        (
            "graph-correction-c",
            SourceNamespace::DirectAthleticsAthlete,
            "103",
        ),
    ] {
        let mut athlete = CanonicalAthlete::new(
            &SchoolId::mint("sch", &[school]),
            "Retained Synthetic Runner",
            GradYear::CO2027,
            Gender::Girls,
            SourceIdentity::new(namespace, source),
        )?;
        athlete.evidence.push(Evidence::parsed(
            SourceRef::new(school, Some(format!("https://{school}.test/{source}"))),
            "2026-10-14",
        ));
        members.push(athlete);
    }
    store.append_many(Table::Athletes, &members)?;
    let mut births = Vec::new();
    for (index, member) in members.iter().enumerate() {
        births.push(writer.establish_source_bound(
            &store,
            &format!("graph-correction-birth-{index}"),
            &member.id.cast(),
            store.snapshot().revision(),
            &writer.head(&store)?,
        )?);
    }
    let first_key = members
        .first()
        .and_then(|row| row.source.clone())
        .ok_or("first primary")?;
    let last_key = members
        .get(2)
        .and_then(|row| row.source.clone())
        .ok_or("last primary")?;
    let first = members.first_mut().ok_or("first")?;
    first
        .identity_attestations
        .push(claim(&first_key, "correction-a-publisher")?);
    let middle = members.get_mut(1).ok_or("middle")?;
    middle.add_identity(first_key.clone());
    middle.add_identity(last_key.clone());
    middle
        .identity_attestations
        .push(claim(&first_key, "correction-b-first-publisher")?);
    middle
        .identity_attestations
        .push(claim(&last_key, "correction-b-last-publisher")?);
    members
        .get_mut(2)
        .ok_or("last")?
        .identity_attestations
        .push(claim(&last_key, "correction-c-publisher")?);
    store.append_many(Table::Athletes, &members)?;
    let first_pair = [members[0].id.cast(), members[1].id.cast()];
    let last_pair = [members[1].id.cast(), members[2].id.cast()];
    let (first_case, first_verdict) =
        context::reviewed_fixture::review(&store, &first_pair, "same_person")?;
    let (last_case, last_verdict) =
        context::reviewed_fixture::review(&store, &last_pair, "same_person")?;
    store.replace_many(Table::ReviewCases, &[first_case.clone(), last_case.clone()])?;
    store.replace_many(Table::IdentityVerdicts, &[first_verdict, last_verdict])?;
    for (operation, case, other) in [
        ("graph-correction-ab", first_case.id.clone(), 1),
        ("graph-correction-bc", last_case.id, 2),
    ] {
        let mut public = vec![
            births[0].public_id().clone(),
            births[other].public_id().clone(),
        ];
        public.sort();
        writer.apply_reviewed_identity(
            &store,
            operation,
            &ReviewedIdentityRequestV1::Merge {
                case_id: case,
                canonical: births[0].public_id().clone(),
                members: public,
            },
            store.snapshot().revision(),
            &writer.head(&store)?,
        )?;
    }
    writer.apply_reviewed_identity(
        &store,
        "graph-correction-redundant-ab",
        &ReviewedIdentityRequestV1::Merge {
            case_id: first_case.id,
            canonical: births[0].public_id().clone(),
            members: vec![births[0].public_id().clone()],
        },
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let candidates = members.iter().map(|row| row.id.cast()).collect::<Vec<_>>();
    let (case, verdict) =
        context::reviewed_fixture::review(&store, &candidates, "different_person")?;
    store.replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
    store.replace_many(Table::IdentityVerdicts, &[verdict])?;
    let accepted = authority::accept_reviewed(
        &context::Inputs::capture(&store.snapshot())?,
        &case.id,
        "2026-10-14",
    )?;
    check!(eq; accepted.record().kind, AppliedIdentityKind::DifferentPerson);
    check!(eq; accepted.record().members.len(), 3);
    let mut affected = members
        .iter()
        .map(|row| format!("athletes:{}", row.id))
        .collect::<Vec<_>>();
    affected.extend(later_page_results(&store, &members)?);
    affected.sort();
    Ok(Fixture {
        directory,
        store,
        writer,
        births,
        members,
        case: case.id,
        affected,
    })
}

pub(super) fn split(fixture: &Fixture) -> TestResult<ReviewedIdentityRequestV1> {
    let mut partitions = fixture
        .members
        .iter()
        .zip(&fixture.births)
        .map(|(member, birth)| {
            Ok(BindingPartition {
                representative: member.id.cast(),
                binding_ids: vec![context::reviewed_fixture::binding_id(
                    &fixture.store,
                    birth.head().sequence,
                )?],
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    partitions.sort_by(|left, right| left.binding_ids.cmp(&right.binding_ids));
    Ok(ReviewedIdentityRequestV1::Split {
        case_id: fixture.case.clone(),
        canonical: fixture.births[0].public_id().clone(),
        revoke_merges: vec![4, 5, 6],
        partitions,
        unresolved_bindings: Vec::new(),
        affected_record_keys: fixture.affected.clone(),
    })
}

pub(super) fn revoke(fixture: &Fixture) -> ReviewedIdentityRequestV1 {
    ReviewedIdentityRequestV1::Revoke {
        case_id: fixture.case.clone(),
        canonical: fixture.births[0].public_id().clone(),
        revoke_merges: vec![4],
        affected_record_keys: fixture.affected.clone(),
    }
}

fn claim(subject: &SourceIdentity, publisher: &str) -> TestResult<IdentityAttestation> {
    let bytes = serde_json::to_vec(
        &serde_json::json!({ "synthetic": true, "publisher": publisher, "subject": subject }),
    )?;
    let claim = IdentityAttestation {
        subject: subject.clone(),
        source: SourceRef::new(publisher, Some(format!("https://{publisher}.test/capture"))),
        capture_sha256: format!("{:x}", Sha256::digest(bytes)),
        acquired_at: "2026-10-14T00:00:00Z".into(),
        source_family: publisher.into(),
        upstream_producer: publisher.into(),
        subject_locator: "subject".into(),
        qualification: AttestationQualification::IndependentPublished,
    };
    claim.validate()?;
    Ok(claim)
}

fn later_page_results(store: &Store, members: &[CanonicalAthlete]) -> TestResult<Vec<String>> {
    let unrelated = subject("graph-correction-unrelated", "109")?;
    store.append(Table::Athletes, &unrelated)?;
    let event = EventId::mint("event", &["graph-correction-later"]);
    let meet = MeetId::mint("meet", &["graph-correction-later"]);
    let team = TeamId::mint("team", &["graph-correction-later"]);
    let performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &unrelated.id,
            event: &event,
            meet: &meet,
            date: "2026-10-14",
            source_key: "graph-correction-physical-variants",
        },
        PerformanceResult {
            team: &team,
            mark: Mark::Raw("12.10".into()),
            wind_mps: None,
            place: Some(1),
            adjustment: None,
        },
    )?;
    let relay = RelayResult::new(
        &team,
        &event,
        &meet,
        Mark::Raw("48.10".into()),
        "graph-correction-physical-variants",
    );
    let mut performances = Vec::new();
    let mut relays = Vec::new();
    for ordinal in 0..520 {
        let mut row = performance.clone();
        row.mark = Mark::Raw(format!("variant-{ordinal}"));
        performances.push(row);
        let mut row = relay.clone();
        row.members =
            vec![RelayMember::new(1, format!("unrelated-{ordinal}"))
                .with_athlete(unrelated.id.clone())];
        relays.push(row);
    }
    let mut later = performance.clone();
    later.athlete = members.first().ok_or("member")?.id.clone();
    performances.push(later);
    let mut later = relay.clone();
    later.members = vec![RelayMember::new(1, "retained later-page member")
        .with_athlete(members.get(2).ok_or("member")?.id.clone())];
    relays.push(later);
    store.append_many(Table::Performances, &performances)?;
    store.append_many(Table::RelayResults, &relays)?;
    Ok(vec![
        format!("performances:{}", performance.id),
        format!("relay_results:{}", relay.id),
    ])
}

pub(super) fn facts(store: &Store) -> TestResult<String> {
    Ok(store.snapshot().tables_digest(&[
        Table::Athletes,
        Table::SourceObservations,
        Table::Performances,
        Table::RelayResults,
    ])?)
}
