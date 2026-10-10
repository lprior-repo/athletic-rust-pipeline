use super::*;

use crate::athlete_packet::packet as athlete_packet;
use census_domain::model::serialized_digest;
use census_domain::model::{AttestationQualification, IdentityAttestation};

#[test]
fn canary_11_swapping_which_subject_owns_the_cohort_fact_changes_review_identity() -> TestResult {
    let mut owns_2027 = athlete("Jordan Smith", Gender::Boys, GradYear::CO2027);
    observed(&mut owns_2027, 11, 2025)?;
    let mut owns_2026 = athlete(
        "Jordan Smith",
        Gender::Boys,
        GradYear::new(2026).ok_or("a grad year in range")?,
    );
    observed(&mut owns_2026, 12, 2025)?;
    check!(ne; owns_2027.id, owns_2026.id, "the two sides are distinct rows");
    let mut group = vec![owns_2027.id.to_string(), owns_2026.id.to_string()];
    group.sort();

    let first = athlete_packet(&case_for(&owns_2027), &owns_2027, &owns_2026, &group)?;
    let second = athlete_packet(&case_for(&owns_2026), &owns_2026, &owns_2027, &group)?;

    check!(eq; stated(&first, "census", "side_a_grad_year"), Some("2027".to_string()));
    check!(eq; stated(&first, "census", "side_b_grad_year"), Some("2026".to_string()));
    check!(eq; stated(&second, "census", "side_a_grad_year"), Some("2026".to_string()));
    check!(eq; stated(&second, "census", "side_b_grad_year"), Some("2027".to_string()));
    check!(eq; stated(&first, "census", "candidate_ids"), Some(group.join(", ")));
    check!(eq; stated(&second, "census", "candidate_ids"), Some(group.join(", ")));
    check!(
        serialized_digest(&first)? != serialized_digest(&second)?,
        "which subject owns the 2027 cohort fact is part of the review evidence identity"
    );
    Ok(())
}

fn reordered_fact_set(reverse: bool) -> TestResult<(CanonicalAthlete, CanonicalAthlete)> {
    let primary = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");
    let mut subject = CanonicalAthlete::new(
        &school(),
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        primary,
    );
    let mut peer = athlete("Jordan Smith", Gender::Girls, GradYear::CO2027);
    let mut observations = vec![(10, 2024), (11, 2025)];
    let mut links = vec![
        (SourceNamespace::TfrrsAthlete, "77", None),
        (
            SourceNamespace::AthleticNet {
                kind: "athlete".to_string(),
            },
            "998877",
            Some("https://www.athletic.net/athlete/998877/track-and-field"),
        ),
    ];
    let mut attestations = vec![
        (SourceNamespace::TfrrsAthlete, "77", "/athletes/77"),
        (
            SourceNamespace::AthleticNet {
                kind: "athlete".to_string(),
            },
            "998877",
            "/athlete/998877",
        ),
    ];
    if reverse {
        observations.reverse();
        links.reverse();
        attestations.reverse();
    }
    for (grade, school_year) in observations {
        observed(&mut subject, grade, school_year)?;
    }
    for (namespace, id, url) in links {
        known_as(&mut subject, namespace, id, url);
    }
    for (namespace, id, locator) in attestations {
        attested(&mut subject, namespace, id, locator);
    }
    observed(&mut peer, 11, 2025)?;
    Ok((subject, peer))
}

fn attested(athlete: &mut CanonicalAthlete, namespace: SourceNamespace, id: &str, locator: &str) {
    athlete.identity_attestations.push(IdentityAttestation {
        subject: SourceIdentity::new(namespace, id),
        source: SourceRef::new(
            "milesplit-capture",
            Some(format!("https://www.milesplit.com/athletes/{id}")),
        ),
        capture_sha256: "0".repeat(64),
        acquired_at: "2026-09-30T12:00:00Z".to_string(),
        source_family: "milesplit".to_string(),
        upstream_producer: "milesplit-profile".to_string(),
        subject_locator: locator.to_string(),
        qualification: AttestationQualification::IndependentPublished,
    });
}

#[test]
fn canary_12_reordering_an_unordered_fact_set_keeps_review_identity() -> TestResult {
    let (forward_subject, forward_peer) = reordered_fact_set(false)?;
    let (reverse_subject, reverse_peer) = reordered_fact_set(true)?;
    check!(eq; forward_subject.id, reverse_subject.id, "a reordered fact set is the same row");
    check!(eq; forward_peer.id, reverse_peer.id);
    check!(
        ne;
        forward_subject.observed_grades,
        reverse_subject.observed_grades,
        "the two fact sets really are stored in different orders"
    );
    check!(
        ne;
        forward_subject.identity_attestations,
        reverse_subject.identity_attestations,
        "the two attestation sets really are stored in different orders"
    );

    let mut group = vec![forward_subject.id.to_string(), forward_peer.id.to_string()];
    group.sort();
    let forward = athlete_packet(
        &case_for(&forward_subject),
        &forward_subject,
        &forward_peer,
        &group,
    )?;
    let reverse = athlete_packet(
        &case_for(&reverse_subject),
        &reverse_subject,
        &reverse_peer,
        &group,
    )?;
    check!(eq; forward.evidence, reverse.evidence, "an unordered fact set states one evidence list");
    check!(
        eq;
        serialized_digest(&forward)?,
        serialized_digest(&reverse)?,
        "reordering an unordered fact set keeps the review evidence identity"
    );
    Ok(())
}
