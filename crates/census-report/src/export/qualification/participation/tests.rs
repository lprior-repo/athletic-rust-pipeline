use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalPerformance, Evidence, GradYear, ProfileDate, SourceRef,
};
use census_domain::participation_candidates::{CandidateOutcome, ParticipationReason};
use census_store::Table;
use serde_json::{json, Value};

mod fixture;
mod groups;
mod historical;
mod profile;
mod refusals;
use fixture::Fixture;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn row(
    token: PinnedParticipation,
    id: &census_domain::model::PerformanceId,
) -> TestResult<PinnedParticipationRow> {
    let (_, _, _, mut rows, _) = token.into_parts();
    Ok(rows.remove(id).ok_or("assessment missing")?)
}

#[tokio::test]
async fn authentic_source_does_not_gain_high_school_status() -> TestResult {
    let fixture = Fixture::prepare(None).await?;
    let (outcome, reasons, references, _) =
        row(fixture.assess()?, &fixture.performance.id)?.into_parts();
    assert_eq!(outcome, CandidateOutcome::Unresolved);
    assert!(reasons.contains(ParticipationReason::HighSchoolStatusMissing));
    assert!(reasons.contains(ParticipationReason::SummerBoundaryUnresolved));
    assert!(references
        .iter()
        .any(|reference| reference.starts_with("acquisition/")));
    Ok(())
}

#[tokio::test]
async fn synthetic_scholastic_source_retains_archive_but_lacks_independent_status_and_window(
) -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let token = fixture.assess()?;
    let payload = serde_json::to_value(&token.replay)?;
    assert_eq!(payload["version"], 2);
    assert!(payload["independent"]["entries"]
        .as_object()
        .ok_or("independent entries")?
        .is_empty());
    let payload = &payload["retained"];
    assert_eq!(
        payload["profile_id"],
        serde_json::to_value(fixture.profile.id()?)?
    );
    assert!(payload["journal"]["entries"]
        .as_array()
        .ok_or("objects")?
        .iter()
        .any(|object| object["phase"] == census_crawl::milesplit::OWNED_MEET_PHASE));
    assert!(payload["journal"]["entries"]
        .as_array()
        .ok_or("objects")?
        .iter()
        .any(|object| object["key"]
            .as_str()
            .is_some_and(|key| key.starts_with("metadata/") && key.ends_with("/0"))));
    let (outcome, reasons, references, gaps) = row(token, &fixture.performance.id)?.into_parts();
    assert_eq!(outcome, CandidateOutcome::Unresolved, "{gaps:?}");
    assert_eq!(
        reasons.iter().collect::<Vec<_>>(),
        vec![
            ParticipationReason::HighSchoolStatusMissing,
            ParticipationReason::SummerBoundaryUnresolved,
        ]
    );
    assert!(gaps.is_empty());
    assert!(references.windows(2).all(|pair| pair[0] < pair[1]));
    Ok(())
}

#[tokio::test]
async fn synthetic_explicit_non_school_status_stays_excluded_and_auditable() -> TestResult {
    for (label, reason) in [
        ("Collegiate", ParticipationReason::CollegiateParticipation),
        (
            "Middle School",
            ParticipationReason::MiddleSchoolParticipation,
        ),
        (
            "Post Graduation",
            ParticipationReason::PostGraduationParticipation,
        ),
    ] {
        let fixture = Fixture::prepare(Some(label)).await?;
        let (outcome, reasons, references, _) =
            row(fixture.assess()?, &fixture.performance.id)?.into_parts();
        assert_eq!(outcome, CandidateOutcome::Excluded);
        assert!(reasons.contains(reason));
        assert!(!references.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn synthetic_club_open_and_unattached_do_not_create_window_endpoints() -> TestResult {
    for label in ["Club", "Open", "Unattached"] {
        let fixture = Fixture::prepare(Some(label)).await?;
        let (outcome, reasons, _, _) =
            row(fixture.assess()?, &fixture.performance.id)?.into_parts();
        assert_eq!(outcome, CandidateOutcome::Unresolved);
        assert!(reasons.contains(ParticipationReason::SummerBoundaryUnresolved));
    }
    Ok(())
}

#[tokio::test]
async fn unverified_supplementary_window_remains_in_physical_closure_after_replacement(
) -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let original = fixture
        .store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .find(|row| row.id == fixture.performance.athlete)
        .ok_or("athlete")?;
    let mut changed = original.clone();
    changed.evidence.push(Evidence::derived(
        SourceRef::id("synthetic-window"),
        "2026-10-01",
        "eligibility window ended before this performance; unverified synthetic evidence",
    ));
    fixture.store.append(Table::Athletes, &changed)?;
    fixture.store.append(Table::Athletes, &original)?;
    let (outcome, _, references, gaps) =
        row(fixture.assess()?, &fixture.performance.id)?.into_parts();
    assert_eq!(outcome, CandidateOutcome::Unresolved);
    assert!(gaps.contains(&ParticipationDiagnostic::UnverifiedAdditionalEvidence));
    let expected = format!(
        "canonical_evidence:{}",
        serialized_digest(changed.evidence.last().ok_or("supplement")?)?
    );
    assert!(references.contains(&expected));
    Ok(())
}

#[tokio::test]
async fn appended_unknown_field_cannot_hide_in_an_otherwise_owned_note() -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let mut athlete = fixture
        .store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .find(|row| row.id == fixture.performance.athlete)
        .ok_or("athlete")?;
    let evidence = athlete
        .evidence
        .iter_mut()
        .find(|evidence| {
            evidence
                .note
                .as_ref()
                .is_some_and(|note| note.contains("best_mark_scope"))
        })
        .ok_or("owned note")?;
    let mut note: Value = serde_json::from_str(evidence.note.as_deref().ok_or("note")?)?;
    note["eligibility_window"] = json!({"end":"2026-01-01"});
    evidence.note = Some(note.to_string());
    fixture.store.append(Table::Athletes, &athlete)?;
    let (outcome, _, _, gaps) = row(fixture.assess()?, &fixture.performance.id)?.into_parts();
    assert_eq!(outcome, CandidateOutcome::Unresolved);
    assert!(gaps.contains(&ParticipationDiagnostic::UnverifiedAdditionalEvidence));
    Ok(())
}

#[tokio::test]
async fn reasons_union_is_empty_identity_commutative_associative_and_idempotent() -> TestResult {
    let mut samples =
        vec![census_domain::participation_candidates::ParticipationReasons::default()];
    for label in [None, Some("Collegiate"), Some("Club")] {
        let fixture = Fixture::prepare(label).await?;
        let (_, reasons, _, _) = row(fixture.assess()?, &fixture.performance.id)?.into_parts();
        samples.push(reasons);
    }
    for a in &samples {
        assert_eq!(a.union(Default::default()), *a);
        assert_eq!(a.union(*a), *a);
        for b in &samples {
            assert_eq!(a.union(*b), b.union(*a));
            for c in &samples {
                assert_eq!(a.union(*b).union(*c), a.union(b.union(*c)));
            }
        }
    }
    Ok(())
}
