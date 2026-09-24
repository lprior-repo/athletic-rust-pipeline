//! Source identity survival across a merge: the reverse map a canonical row is asked for.
//!
//! A canonical row is not only one subject's fields. It is the place a reader asks "which provider
//! objects are this subject?" — [`identity_in`](census_domain::model::CanonicalAthlete::identity_in),
//! and the §31 join behind it — so a merge has to be *reversible* in exactly that sense: after
//! absorbing a second observation, every provider identity either side carried is still on the row,
//! the row cites no provider object neither side named, and a row whose key collided with a different
//! subject absorbs no identity at all — the finding it retains names both sides' sources instead,
//! which is the evidence an operator splits the pair back apart from.
//!
//! [`super::laws_unions`] says merge order cannot change the element sets; these say what the set *is*.

use super::*;
use census_domain::model::{NaturalKey, CANONICAL_ID_COLLISION_FAMILY};

/// Every identity both sides of a merge carried, in one list.
fn both_sides(first: &[SourceIdentity], second: &[SourceIdentity]) -> Vec<SourceIdentity> {
    let mut union = first.to_vec();
    union.extend_from_slice(second);
    union
}

proptest! {
    #![proptest_config(law_config())]

    /// The merged row carries exactly the union: nothing a source published is dropped, and the row
    /// cannot cite a provider object that neither observation named.
    #[test]
    fn an_athlete_merge_keeps_exactly_the_union_of_the_source_identities(
        base in athlete(),
        left in peer_identity(SourceNamespace::MilesplitAthlete),
        right in peer_identity(SourceNamespace::TfrrsAthlete),
    ) {
        let mut first = base.clone();
        first.source_identities.push(left);
        let mut second = base;
        second.source_identities.push(right);

        let union = both_sides(&first.source_identities, &second.source_identities);
        let mut merged = first.clone();
        merged.merge(second.clone());

        prop_assert!(
            same_members(&merged.source_identities, &union),
            "{:?} is not the union {:?}",
            merged.source_identities,
            union
        );
        for identity in &merged.source_identities {
            prop_assert!(
                first.source_identities.contains(identity) || second.source_identities.contains(identity),
                "{:?} appeared on the row without either observation naming it",
                identity
            );
        }
    }

    /// The same law at the other shape that holds an identity list: the union is one helper, but each
    /// row type calls it for its own field, so the school row is the second witness.
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

    /// The row is also the reverse map: every namespace either observation named answers from the
    /// merged row, and answers with an identity one of the two sides actually carried.
    #[test]
    fn every_namespace_a_side_named_answers_from_the_merged_athlete(
        base in athlete(),
        left in peer_identity(SourceNamespace::MilesplitAthlete),
        right in peer_identity(SourceNamespace::TfrrsAthlete),
    ) {
        let mut first = base.clone();
        first.source_identities.push(left);
        let mut second = base;
        second.source_identities.push(right);

        let union = both_sides(&first.source_identities, &second.source_identities);
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

    /// A refused merge absorbs nothing: the survivor keeps the identity list it held, and the finding
    /// it retains names *both* sides' provider objects.
    #[test]
    fn a_refused_athlete_merge_keeps_its_identities_and_names_both_sides(
        base in athlete(),
        other_name in word(20),
        left in peer_identity(SourceNamespace::MilesplitAthlete),
        right in peer_identity(SourceNamespace::TfrrsAthlete),
    ) {
        let mut first = base.clone();
        first.source_identities.push(left);
        let mut incoming = base;
        incoming.canonical_name = other_name;
        incoming.known_names = vec![incoming.canonical_name.clone()];
        incoming.source_identities.push(right);
        prop_assume!(!first.same_natural_key(&incoming));

        let mut merged = first.clone();
        merged.merge(incoming.clone());

        prop_assert!(
            same_members(&merged.source_identities, &first.source_identities),
            "{:?} is not what the survivor held",
            merged.source_identities
        );
        let finding = merged
            .retained_conflicts
            .iter()
            .find(|conflict| conflict.family == CANONICAL_ID_COLLISION_FAMILY);
        prop_assert!(finding.is_some(), "a collision left no finding behind");
        if let Some(finding) = finding {
            for identity in both_sides(&first.source_identities, &incoming.source_identities) {
                let rendered = format!("{}:{}", identity.namespace, identity.id);
                prop_assert!(
                    finding.detail.contains(&rendered),
                    "the finding does not name {rendered}: {}",
                    finding.detail
                );
            }
        }

        // The store re-merges a row on every read of its table, so the refusal has to be stable: a
        // second read of the same observation adds no second finding and still moves no identity.
        merged.merge(incoming);
        prop_assert_eq!(merged.retained_conflicts.len(), 1);
        prop_assert!(same_members(&merged.source_identities, &first.source_identities));
    }

    /// A source that renumbers its own object hands over two ids under one namespace. Both stay on
    /// the row — the earlier join is never overwritten — and the reverse lookup answers with one of
    /// them rather than with a third.
    #[test]
    fn two_ids_under_one_namespace_both_survive_the_merge(
        base in athlete(),
        left in peer_identity(SourceNamespace::MilesplitAthlete),
        right in peer_identity(SourceNamespace::MilesplitAthlete),
    ) {
        prop_assume!(left != right);
        let mut first = base.clone();
        first.source_identities.push(left);
        let mut second = base;
        second.source_identities.push(right);

        let mut merged = first.clone();
        merged.merge(second);

        prop_assert_eq!(merged.source_identities.len(), 2);
        let found = merged.identity_in(&SourceNamespace::MilesplitAthlete);
        prop_assert!(found.is_some());
        if let Some(found) = found {
            prop_assert!(
                merged.source_identities.contains(found),
                "{found:?} is not one of the ids the row holds"
            );
        }
    }
}
