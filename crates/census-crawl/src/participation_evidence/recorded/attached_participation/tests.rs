use super::*;
use crate::participation_evidence::{
    IndependentParticipationPacketV1, RetainedBodyEncodingV1, RetainedHtmlCaptureV1,
    RetainedHttpProvenanceV1, SubjectOwnershipV1,
};
use census_domain::model::StoreIdentity;
use chrono::{DateTime, Utc};

mod adversarial;
mod fixture;
mod genuine;
mod targets;

pub(in crate::participation_evidence::recorded) use adversarial::verify_adversarial;
pub(in crate::participation_evidence::recorded) use genuine::verify_genuine;
pub(in crate::participation_evidence::recorded) use targets::verify_targets;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn genuine_status_packet_keeps_2026_documentary_authority_and_event_alias_gap() -> TestResult {
    let packet = fixture::packet()?;
    let checked = packet.read(StoreIdentity::parse(&"a".repeat(64))?, 1)?;
    check!(eq; checked.cohort().get(), 2026);
    check!(eq; checked.day().as_str(), "2026-05-15");
    check!(eq; checked.school_year().get(), 2025);
    check!(matches!(
        checked.ownership(),
        SubjectOwnershipV1::DocumentOnly(_)
    ));
    check!(eq; checked.competition(), "MHSAA Regionals");
    check!(eq; checked.venue(), "Milford HS");
    check!(eq; join::season("MHSAA LP Region 09-1 @ Milford", "Highland Milford High School", &checked), DocumentarySeasonApplicabilityV1::CompetitionAndVenueMismatch);
    Ok(())
}

#[test]
fn exact_semantic_labels_and_each_mismatch_are_distinct() -> TestResult {
    let checked = fixture::packet()?.read(StoreIdentity::parse(&"a".repeat(64))?, 1)?;
    for (competition, venue, expected) in [
        (
            "MHSAA Regionals",
            "Milford HS",
            DocumentarySeasonApplicabilityV1::Exact,
        ),
        (
            "Other competition",
            "Milford HS",
            DocumentarySeasonApplicabilityV1::CompetitionMismatch,
        ),
        (
            "MHSAA Regionals",
            "Other venue",
            DocumentarySeasonApplicabilityV1::VenueMismatch,
        ),
        (
            "Other competition",
            "Other venue",
            DocumentarySeasonApplicabilityV1::CompetitionAndVenueMismatch,
        ),
    ] {
        check!(eq; join::season(competition, venue, &checked), expected);
    }
    Ok(())
}

#[test]
fn canonical_documentary_venue_preserves_original_label() -> TestResult {
    let capture = fixture::result()?;
    let row = crate::milesplit::SourceRowLocator {
        ordinal: 527,
        byte_offset: 80520,
        byte_length: 100,
    };
    let document = crate::participation_evidence::result_ownership::read_retained_html_result_rows(
        &capture,
        &[row],
    )?;
    check!(eq; crate::participation_evidence::result_context::documentary_result_venue(&document)?, "Highland Milford High School");
    check!(eq; document.parsed_page().meet.name, "MHSAA LP Region 09-1 @ Milford");
    Ok(())
}

#[test]
fn contrary_result_grade_retains_source_cohort_conflict() -> TestResult {
    let checked = fixture::packet()?.read(StoreIdentity::parse(&"a".repeat(64))?, 1)?;
    let senior = census_domain::model::Grade::new(12).ok_or("senior grade")?;
    let junior = census_domain::model::Grade::new(11).ok_or("junior grade")?;
    check!(eq; join::cohort_agreement(Some(senior), checked.school_year(), checked.cohort()), DocumentaryCohortAgreementV1::Agreed);
    check!(eq; join::cohort_agreement(Some(junior), checked.school_year(), checked.cohort()), DocumentaryCohortAgreementV1::Conflicting);
    check!(eq; join::cohort_agreement(None, checked.school_year(), checked.cohort()), DocumentaryCohortAgreementV1::Agreed);
    Ok(())
}
