use crate::model::{ProfileDate, SourceNamespace};

use super::fixture::*;
use super::*;

#[test]
fn provider_id_reuse_is_never_an_automatic_merge() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("a", "7"))?;
    let _second = fixture.append(establish("b", "7"))?;
    check!(eq; fixture.project().err(), Some(HistoryError::SourceReuse));
    Ok(())
}

#[test]
fn explicit_disjoint_reuse_retains_two_distinct_people_and_dated_bindings() -> TestResult {
    let mut fixture = Fixture::new()?;
    let mut first = binding("a", "7");
    first.effective = closed("2020-01-01", "2020-12-31")?;
    let _first = fixture.append(HistoryRequest::Establish {
        binding: first,
        evidence: evidence("source:a", CandidateEvidenceKind::SourceBound, &["a"]),
    })?;
    let mut second = binding("b", "7");
    second.effective = closed("2026-01-01", "2026-12-31")?;
    second.supersedes = Some("binding:a".into());
    let source = second.source.clone();
    let _second = fixture.append(HistoryRequest::Establish {
        binding: second,
        evidence: evidence("source:b", CandidateEvidenceKind::SourceBound, &["b"]),
    })?;
    let projection = fixture.project()?;
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(FIRST)?)?,
        PublicResolution::Canonical(successor(FIRST, "a", 1, 0)?));
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(SECOND)?)?,
        PublicResolution::Canonical(successor(SECOND, "b", 2, 0)?));
    check!(eq; projection.source_bindings_at(&source, &ProfileDate::parse("2020-12-31")?)?, vec!["binding:a"]);
    check!(eq; projection.source_bindings_at(&source, &ProfileDate::parse("2026-01-01")?)?, vec!["binding:b"]);
    check!(eq; projection.source_bindings_at(&source, &ProfileDate::parse("2023-01-01")?)?, Vec::<&str>::new());
    check!(eq; projection.resolve_binding("binding:a")?, BindingResolution::Resolved {
        identity: PublicResolution::Canonical(successor(FIRST, "a", 1, 0)?),
        established_at: 1, superseded_by: Some("binding:b".into()),
    });
    Ok(())
}

#[test]
fn touching_inclusive_binding_intervals_are_a_reuse_conflict() -> TestResult {
    let mut fixture = Fixture::new()?;
    for (label, from, through, supersedes) in [
        ("a", "2020-01-01", "2026-07-31", None),
        ("b", "2026-07-31", "2027-07-31", Some("binding:a")),
    ] {
        let mut claim = binding(label, "7");
        claim.effective = closed(from, through)?;
        claim.supersedes = supersedes.map(str::to_owned);
        let _births = fixture.append(HistoryRequest::Establish {
            binding: claim,
            evidence: evidence(
                &format!("source:{label}"),
                CandidateEvidenceKind::SourceBound,
                &[label],
            ),
        })?;
    }
    check!(eq; fixture.project().err(), Some(HistoryError::SourceReuse));
    Ok(())
}

#[test]
fn unknown_effective_scope_cannot_certify_reuse_or_a_dated_source_lookup() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("a", "7"))?;
    let source = binding("a", "7").source;
    check!(eq; fixture.project()?.source_bindings_at(&source, &ProfileDate::parse("2026-07-31")?).err(),
        Some(HistoryError::BindingConflict));
    let mut next = binding("b", "7");
    next.supersedes = Some("binding:a".into());
    next.effective = closed("2027-01-01", "2027-12-31")?;
    let _second = fixture.append(HistoryRequest::Establish {
        binding: next,
        evidence: evidence("source:b", CandidateEvidenceKind::SourceBound, &["b"]),
    })?;
    check!(eq; fixture.project().err(), Some(HistoryError::SourceReuse));
    Ok(())
}

#[test]
fn identical_raw_ids_in_different_namespaces_remain_distinct() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("a", "7"))?;
    let mut other = binding("b", "7");
    other.source.namespace = SourceNamespace::MilesplitAthlete;
    let _second = fixture.append(HistoryRequest::Establish {
        binding: other,
        evidence: evidence("source:b", CandidateEvidenceKind::SourceBound, &["b"]),
    })?;
    check!(eq; fixture.project()?.resolve_public(&PublicAthleteId::parse(SECOND)?)?,
        PublicResolution::Canonical(successor(SECOND, "b", 2, 0)?));
    Ok(())
}

#[test]
fn accepted_match_can_add_a_binding_without_changing_public_id() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("a", "1"))?;
    let _births = fixture.append(HistoryRequest::Bind {
        owner: PublicAthleteId::parse(FIRST)?,
        binding: binding("b", "2"),
        evidence: evidence("match:ab", CandidateEvidenceKind::SamePerson, &["a", "b"]),
    })?;
    check!(eq; fixture.project()?.resolve_binding("binding:b")?, BindingResolution::Resolved {
        identity: PublicResolution::Canonical(successor(FIRST, "a", 1, 0)?),
        established_at: 2, superseded_by: None,
    });
    Ok(())
}

#[test]
fn source_bound_evidence_alone_cannot_add_someone_to_an_existing_person() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("a", "1"))?;
    let _births = fixture.append(HistoryRequest::Bind {
        owner: PublicAthleteId::parse(FIRST)?,
        binding: binding("b", "2"),
        evidence: evidence("source:b", CandidateEvidenceKind::SourceBound, &["b"]),
    })?;
    check!(eq; fixture.project().err(), Some(HistoryError::EvidenceMismatch));
    Ok(())
}

#[test]
fn candidate_reassignment_is_not_inferred_from_a_second_source_claim() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("a", "1"))?;
    let mut duplicate = binding("a", "2");
    duplicate.id = "another-binding".into();
    let _births = fixture.append(HistoryRequest::Establish {
        binding: duplicate,
        evidence: evidence("source:again", CandidateEvidenceKind::SourceBound, &["a"]),
    })?;
    check!(eq; fixture.project().err(), Some(HistoryError::BindingConflict));
    Ok(())
}
