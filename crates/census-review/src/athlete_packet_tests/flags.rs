//! What the store already knows before a model runs: the flags, and the packet facts that carry
//! them.

use super::*;

use crate::athlete_flags::{flags, FlagKind};
use crate::athlete_packet::packet as athlete_packet;

#[test]
fn the_flags_state_what_the_store_already_knows_before_the_model_runs() {
    let (boys, girls, _) = rows();
    let kinds: Vec<FlagKind> = flags(&boys, &girls)
        .into_iter()
        .map(|flag| flag.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            FlagKind::SharedSourceIdentity,
            FlagKind::GradYearEvidenceDiffers,
            FlagKind::GenderDiffers,
            FlagKind::NameSchoolCohortAgree,
        ],
        "the disagreements come first, and the agreement the store already made comes last"
    );

    let case = case_for(&boys);
    let packet = athlete_packet(
        &case,
        &boys,
        &girls,
        &[boys.id.to_string(), girls.id.to_string()],
    );
    let stated_flags: Vec<&str> = packet
        .evidence
        .iter()
        .filter(|fact| fact.field == "flag")
        .map(|fact| fact.value.as_str())
        .collect();
    assert!(
        stated_flags
            .iter()
            .any(|flag| flag.starts_with("shared_source_identity:") && flag.contains("14399169")),
        "the shared provider object is a flag the packet states, not something a model must notice"
    );
    assert!(stated_flags
        .iter()
        .any(|flag| flag.starts_with("gender_differs:")));
    assert!(stated_flags
        .iter()
        .any(|flag| flag.starts_with("grad_year_evidence_differs:")));
}

#[test]
fn provider_objects_that_differ_are_stated_and_never_called_a_shared_one() {
    let (boys, mut girls, _) = rows();
    // One provider, two athlete objects: a later mint that keeps both rows has to be told this, and
    // a model must not have to compare the two ids itself to see it.
    girls
        .source_identities
        .retain(|identity| identity.namespace != SourceNamespace::MilesplitAthlete);
    known_as(
        &mut girls,
        SourceNamespace::MilesplitAthlete,
        "14399170",
        None,
    );

    let stated_flags: Vec<(FlagKind, String)> = flags(&boys, &girls)
        .into_iter()
        .map(|flag| (flag.kind, flag.detail))
        .collect();
    let distinct = stated_flags
        .iter()
        .find(|(kind, _)| *kind == FlagKind::DistinctProviderObjects)
        .map(|(_, detail)| detail.clone())
        .expect("the two objects a provider issued are a flag");
    assert!(
        distinct.contains("14399169") && distinct.contains("14399170"),
        "the flag names both objects: {distinct}"
    );
    assert!(
        !stated_flags
            .iter()
            .any(|(kind, _)| *kind == FlagKind::SharedSourceIdentity),
        "the objects differ, so the provider is not saying the rows are one athlete"
    );

    let case = case_for(&boys);
    let packet = athlete_packet(
        &case,
        &boys,
        &girls,
        &[boys.id.to_string(), girls.id.to_string()],
    );
    assert!(
        packet
            .evidence
            .iter()
            .any(|fact| fact.value.starts_with("distinct_provider_objects:")),
        "the flag reaches the packet, where the model reads it"
    );
}

#[test]
fn rows_that_disagree_on_the_class_are_flagged_and_never_called_an_agreement() {
    let (boys, _, _) = rows();
    let mut later = athlete(
        "Jordan Smith",
        Gender::Boys,
        GradYear::new(2028).expect("2028"),
    );
    observed(&mut later, 10, 2025);
    let kinds: Vec<FlagKind> = flags(&boys, &later)
        .into_iter()
        .map(|flag| flag.kind)
        .collect();
    assert!(kinds.contains(&FlagKind::GradYearDiffers));
    assert!(
        !kinds.contains(&FlagKind::NameSchoolCohortAgree),
        "the agreement flag is read off the rows, not assumed from the pairing"
    );
}
