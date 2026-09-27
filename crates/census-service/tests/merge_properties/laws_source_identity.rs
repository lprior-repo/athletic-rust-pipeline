use super::*;
use census_domain::model::{NaturalKey, CANONICAL_ID_COLLISION_FAMILY};

fn both_sides(first: &[SourceIdentity], second: &[SourceIdentity]) -> Vec<SourceIdentity> {
    let mut union = first.to_vec();
    for identity in second {
        if !union.contains(identity) {
            union.push(identity.clone());
        }
    }
    union
}

fn athlete_identities(row: &CanonicalAthlete) -> Vec<SourceIdentity> {
    row.identities().cloned().collect()
}

proptest! {
    #![proptest_config(law_config())]

    #[test]
    fn an_athlete_merge_keeps_exactly_the_union_of_the_source_identities(
        base in athlete(),
        left in peer_identity(SourceNamespace::MilesplitAthlete),
        right in peer_identity(SourceNamespace::TfrrsAthlete),
    ) {
        let mut first = base.clone();
        first.add_identity(left);
        let mut second = base;
        second.add_identity(right);

        let first_ids = athlete_identities(&first);
        let second_ids = athlete_identities(&second);
        let union = both_sides(&first_ids, &second_ids);
        let mut merged = first.clone();
        merged.merge(second);

        let merged_ids = athlete_identities(&merged);
        prop_assert!(
            same_members(&merged_ids, &union),
            "{:?} is not the union {:?}",
            merged_ids,
            union
        );
        for identity in &merged_ids {
            prop_assert!(
                first_ids.contains(identity) || second_ids.contains(identity),
                "{:?} appeared on the row without either observation naming it",
                identity
            );
        }
    }

    #[test]
    fn a_school_merge_keeps_exactly_the_union_of_the_source_identities(
        base in school(),
        left in peer_identity(SourceNamespace::MilesplitSchool),
        right in peer_identity(SourceNamespace::TfrrsTeam),
    ) {
        let mut first = base.clone();
        first.source_identities.push(left);
        let mut second = base;
        second.source_identities.push(right);

        let union = both_sides(&first.source_identities, &second.source_identities);
        let mut merged = first.clone();
        merged.merge(second.clone());

        prop_assert!(same_members(&merged.source_identities, &union), "{:?} vs {:?}", merged.source_identities, union);
        for identity in &merged.source_identities {
            prop_assert!(
                first.source_identities.contains(identity) || second.source_identities.contains(identity),
                "{:?} appeared on the row without either observation naming it",
                identity
            );
        }
    }

    #[test]
    fn every_namespace_a_side_named_answers_from_the_merged_athlete(
        base in athlete(),
        left in peer_identity(SourceNamespace::MilesplitAthlete),
        right in peer_identity(SourceNamespace::TfrrsAthlete),
    ) {
        let mut first = base.clone();
        first.add_identity(left);
        let mut second = base;
        second.add_identity(right);

        let union = both_sides(&athlete_identities(&first), &athlete_identities(&second));
        let mut merged = first.clone();
        merged.merge(second);

        for identity in &union {
            let found = merged.identity_in(&identity.namespace);
            prop_assert!(
                found.is_some(),
                "{} left the row after the merge",
                identity.namespace
            );
            if let Some(found) = found {
                prop_assert!(
                    union.contains(found),
                    "{:?} answers with an identity neither side carried",
                    found
                );
            }
        }
    }

    #[test]
    fn a_refused_athlete_merge_keeps_its_identities_and_names_both_sides(
        base in athlete(),
        other_name in word(20),
        left in peer_identity(SourceNamespace::MilesplitAthlete),
        right in peer_identity(SourceNamespace::TfrrsAthlete),
    ) {
        let mut first = base.clone();
        first.add_identity(left);
        let mut incoming = base;
        incoming.canonical_name = other_name;
        incoming.known_names = vec![incoming.canonical_name.clone()];
        incoming.add_identity(right);
        prop_assume!(!first.same_natural_key(&incoming));

        let first_ids = athlete_identities(&first);
        let mut merged = first.clone();
        merged.merge(incoming.clone());

        let merged_ids = athlete_identities(&merged);
        prop_assert!(
            same_members(&merged_ids, &first_ids),
            "{:?} is not what the survivor held",
            merged_ids
        );
        let finding = merged
            .retained_conflicts
            .iter()
            .find(|conflict| conflict.family == CANONICAL_ID_COLLISION_FAMILY);
        prop_assert!(finding.is_some(), "a collision left no finding behind");
        if let Some(finding) = finding {
            for identity in both_sides(&first_ids, &athlete_identities(&incoming)) {
                let rendered = format!("{}:{}", identity.namespace, identity.id);
                prop_assert!(
                    finding.detail.contains(&rendered),
                    "the finding does not name {rendered}: {}",
                    finding.detail
                );
            }
        }

        merged.merge(incoming);
        prop_assert_eq!(merged.retained_conflicts.len(), 1);
        prop_assert!(same_members(&athlete_identities(&merged), &first_ids));
    }

    #[test]
    fn two_ids_under_one_namespace_both_survive_the_merge(
        base in athlete(),
        left in peer_identity(SourceNamespace::MilesplitAthlete),
        right in peer_identity(SourceNamespace::MilesplitAthlete),
    ) {
        prop_assume!(left != right);
        let mut first = base.clone();
        first.add_identity(left.clone());
        let mut second = base;
        second.add_identity(right.clone());

        let mut merged = first.clone();
        merged.merge(second);

        prop_assert_eq!(&merged.source, &first.source, "the primary owner never moves");
        prop_assert_eq!(merged.source_links.len(), 2, "the peer ids did not both survive");
        let held = athlete_identities(&merged);
        prop_assert!(held.contains(&left), "{left:?} left the row");
        prop_assert!(held.contains(&right), "{right:?} left the row");
        let found = merged.identity_in(&SourceNamespace::MilesplitAthlete);
        prop_assert!(found.is_some());
        if let Some(found) = found {
            prop_assert!(
                held.contains(found),
                "{found:?} is not one of the ids the row holds"
            );
        }
    }
}
