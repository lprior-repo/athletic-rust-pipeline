use super::*;

#[test]
fn documentary_writer_and_readback_refuse_wrong_actual_outcome_mapping_or_tag() -> TestResult {
    let fixture = Fixture::empty()?;
    let original = view(&fixture, rows()?)?;
    let outcome = outcome(&fixture, &original)?;
    let directory = tempfile::tempdir()?;
    write(directory.path(), &fixture.dataset, &original, &outcome)?;
    let before = std::fs::read(directory.path().join(FILE))?;
    let cases = [
        (
            ParticipationOutcome::Unresolved,
            vec![
                ParticipationDiagnostic::DocumentaryAssessmentEligible,
                ParticipationDiagnostic::SchoolBindingUnresolved,
            ],
        ),
        (
            ParticipationOutcome::Eligible,
            vec![
                ParticipationDiagnostic::DocumentaryAssessmentUnresolved,
                ParticipationDiagnostic::SchoolBindingUnresolved,
            ],
        ),
        (
            ParticipationOutcome::Eligible,
            vec![ParticipationDiagnostic::DocumentaryAssessmentEligible],
        ),
        (
            ParticipationOutcome::Eligible,
            vec![
                ParticipationDiagnostic::DocumentaryAssessmentEligible,
                ParticipationDiagnostic::SchoolBindingUnresolved,
                ParticipationDiagnostic::DocumentaryAssessmentExcluded,
            ],
        ),
    ];
    for (mapped, diagnostics) in cases {
        let mut changed = rows()?;
        let eligible = changed
            .values_mut()
            .find(|row| row.domain_outcome() == CandidateOutcome::EligibleCandidate)
            .ok_or("eligible observation")?;
        eligible.override_assessment_for_test(mapped, diagnostics);
        let changed = view(&fixture, changed)?;
        check!(verify(directory.path(), &fixture.dataset, &changed, &outcome).is_err());
        check!(write(directory.path(), &fixture.dataset, &changed, &outcome).is_err());
        std::fs::write(directory.path().join(FILE), &before)?;
    }
    check!(eq; verify(directory.path(), &fixture.dataset, &original, &outcome)?, 3);
    Ok(())
}

#[test]
fn documentary_writer_refuses_same_performance_id_with_another_actual_assessment() -> TestResult {
    let fixture = Fixture::empty()?;
    let original = view(&fixture, rows()?)?;
    let outcome = outcome(&fixture, &original)?;
    let directory = tempfile::tempdir()?;
    write(directory.path(), &fixture.dataset, &original, &outcome)?;
    let mut changed = rows()?;
    let excluded = changed
        .values()
        .find(|row| row.domain_outcome() == CandidateOutcome::Excluded)
        .ok_or("excluded observation")?;
    let id = excluded.performance_id().clone();
    let replacement = DocumentaryParticipationRow::fixture(
        id.clone(),
        excluded.event_id().clone(),
        excluded.athlete_id().clone(),
        CandidateOutcome::Unresolved,
        excluded.assessment().reasons(),
        excluded.assessment().evidence_refs().to_vec(),
    );
    changed.insert(id, replacement);
    let changed = view(&fixture, changed)?;
    check!(verify(directory.path(), &fixture.dataset, &changed, &outcome).is_err());
    check!(write(directory.path(), &fixture.dataset, &changed, &outcome).is_err());
    Ok(())
}

#[test]
fn documentary_inventory_cannot_be_omitted_or_laundered_as_ordinary_exclusions() -> TestResult {
    let fixture = Fixture::empty()?;
    let view = view(&fixture, rows()?)?;
    let full = outcome(&fixture, &view)?;
    let directory = tempfile::tempdir()?;
    write(directory.path(), &fixture.dataset, &view, &full)?;
    let original = std::fs::read(directory.path().join(FILE))?;
    let mut omitted = outcome(&fixture, &view)?;
    omitted
        .participation_exclusions
        .pop()
        .ok_or("documentary exclusion")?;
    check!(write(directory.path(), &fixture.dataset, &view, &omitted).is_err());
    check!(verify(directory.path(), &fixture.dataset, &view, &omitted).is_err());
    check!(eq; std::fs::read(directory.path().join(FILE))?, original);
    let mut ordinary = outcome(&fixture, &view)?;
    ordinary
        .participation_exclusions
        .first_mut()
        .ok_or("documentary exclusion")?
        .override_reason_for_test(ParticipationExclusionReason::MissingParticipationEvidence);
    check!(write(directory.path(), &fixture.dataset, &view, &ordinary).is_err());
    check!(verify(directory.path(), &fixture.dataset, &view, &ordinary).is_err());
    check!(eq; std::fs::read(directory.path().join(FILE))?, original);
    Ok(())
}

#[test]
fn unsupported_documentary_identity_survives_diagnostics_and_refuses_publication() -> TestResult {
    use census_domain::evidence_subject_attachment::{
        AttachmentDigestV1, AttachmentRecordReferenceV1, AttachmentSubjectRoleV1, AttachmentTextV1,
        RetainedDocumentSubjectReferenceV1,
    };
    let fixture = Fixture::empty()?;
    let original = view(&fixture, rows()?)?;
    let outcome = outcome(&fixture, &original)?;
    let directory = tempfile::tempdir()?;
    write(directory.path(), &fixture.dataset, &original, &outcome)?;
    let before = std::fs::read(directory.path().join(FILE))?;
    let digest = AttachmentDigestV1::parse(&"a".repeat(64))?;
    let record = AttachmentRecordReferenceV1 {
        operation: AttachmentTextV1::parse("synthetic-late-unsupported-group")?,
        digest: digest.clone(),
    };
    let subject = RetainedDocumentSubjectReferenceV1 {
        namespace: AttachmentTextV1::parse("synthetic-documentary")?,
        native_id: None,
        capture: digest.clone(),
        body: digest.clone(),
        interpretation: digest.clone(),
        locator: digest.clone(),
        subject: digest.clone(),
        context: digest,
        role: AttachmentSubjectRoleV1::ResultRow,
    };
    let reason = "unsupported_locator_syntax";
    let unsupported = crate::export::UnsupportedDocumentarySource::fixture(
        record.clone(),
        subject.clone(),
        reason,
    );
    let changed = fixture
        .view()?
        .fixture_with_documentary(rows()?, vec![unsupported]);
    check!(eq; changed.documentary_rows().count(), 3);
    let [diagnostic] = changed.unsupported_documentary() else {
        return Err("exact unsupported identity diagnostic".into());
    };
    check!(eq; diagnostic.record(), &record);
    check!(eq; diagnostic.subject(), &subject);
    check!(eq; diagnostic.reason(), reason);
    check!(changed.require_documentary_publication().is_err());
    check!(write(directory.path(), &fixture.dataset, &changed, &outcome).is_err());
    check!(verify(directory.path(), &fixture.dataset, &changed, &outcome).is_err());
    check!(eq; std::fs::read(directory.path().join(FILE))?, before);
    Ok(())
}
