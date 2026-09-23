//! Idempotency and union commutativity.
//!
//! `Entity::merge` is called once per observation on every read of the store, so absorbing the same
//! observation twice must be indistinguishable from absorbing it once, and the element sets a merge
//! unions must not depend on the order in which the two observations arrived.

use super::*;

proptest! {
    #![proptest_config(law_config())]

    #[test]
    fn school_merge_is_idempotent(school in school()) {
        let mut merged = school.clone();
        merged.merge(school.clone());
        prop_assert_eq!(merged, school);
    }

    #[test]
    fn team_merge_is_idempotent(team in team()) {
        let mut merged = team.clone();
        merged.merge(team.clone());
        prop_assert_eq!(merged, team);
    }

    #[test]
    fn coach_merge_is_idempotent(base in coach(), address in mailbox()) {
        let mut coach = base;
        coach.professional_email = Some(address);
        coach.phone = Some("608-555-0100".to_string());
        let mut merged = coach.clone();
        merged.merge(coach.clone());
        prop_assert_eq!(merged, coach);
    }

    #[test]
    fn athlete_merge_is_idempotent(athlete in athlete()) {
        let mut merged = athlete.clone();
        merged.merge(athlete.clone());
        prop_assert_eq!(merged, athlete);
    }

    #[test]
    fn meet_merge_is_idempotent(meet in meet()) {
        let mut merged = meet.clone();
        merged.merge(meet.clone());
        prop_assert_eq!(merged, meet);
    }

    #[test]
    fn event_merge_is_idempotent(event in event()) {
        let mut merged = event.clone();
        merged.merge(event.clone());
        prop_assert_eq!(merged, event);
    }
}

// ---------------------------------------------------------------------------
// Union commutativity: merge order cannot change the element set
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(law_config())]

    #[test]
    fn school_unions_are_commutative(
        base in school(),
        left in peer_identity(SourceNamespace::MilesplitSchool),
        right in peer_identity(SourceNamespace::TfrrsTeam),
        alias in word(10),
        evidence in evidence(),
    ) {
        let mut first = base.clone();
        first.source_identities.push(left);
        first.aliases.push(alias);
        let mut second = base;
        second.source_identities.push(right);
        second.evidence.push(evidence);

        let mut merged = first.clone();
        merged.merge(second.clone());
        let mut reversed = second;
        reversed.merge(first);

        prop_assert!(
            same_members(&merged.source_identities, &reversed.source_identities),
            "{:?} vs {:?}", merged.source_identities, reversed.source_identities
        );
        prop_assert!(same_members(&merged.aliases, &reversed.aliases));
        prop_assert!(same_members(&merged.evidence, &reversed.evidence));
    }

    #[test]
    fn team_unions_are_commutative(
        base in team(),
        left in peer_identity(SourceNamespace::TimerTeam { provider: "pttiming".to_string() }),
        right in peer_identity(SourceNamespace::MilesplitTeam),
        evidence in evidence(),
    ) {
        let mut first = base.clone();
        first.source_identities.push(left);
        let mut second = base;
        second.source_identities.push(right);
        second.evidence.push(evidence);

        let mut merged = first.clone();
        merged.merge(second.clone());
        let mut reversed = second;
        reversed.merge(first);

        prop_assert!(same_members(&merged.source_identities, &reversed.source_identities));
        prop_assert!(same_members(&merged.evidence, &reversed.evidence));
    }

    #[test]
    fn meet_unions_are_commutative(
        base in meet(),
        other_sport in sport(),
        left in peer_identity(SourceNamespace::TimerMeet { provider: "accurace".to_string() }),
        right in peer_identity(SourceNamespace::MilesplitMeet),
        url in word(20),
        evidence in evidence(),
    ) {
        let mut first = base.clone();
        first.source_identities.push(left);
        first.sports.push(other_sport);
        let mut second = base;
        second.source_identities.push(right);
        second.source_urls.push(format!("https://example.org/{url}"));
        second.evidence.push(evidence);

        let mut merged = first.clone();
        merged.merge(second.clone());
        let mut reversed = second;
        reversed.merge(first);

        prop_assert!(same_members(&merged.source_identities, &reversed.source_identities));
        prop_assert!(same_members(&merged.sports, &reversed.sports));
        prop_assert!(same_members(&merged.source_urls, &reversed.source_urls));
        prop_assert!(same_members(&merged.evidence, &reversed.evidence));
    }

    #[test]
    fn athlete_unions_are_commutative(
        base in athlete(),
        other_name in word(20),
        left in peer_identity(SourceNamespace::MilesplitAthlete),
        right in peer_identity(SourceNamespace::TfrrsAthlete),
        other_sport in sport(),
    ) {
        let mut first = base.clone();
        // Replace rather than push: a pushed name could repeat one the row already carries, and
        // `union_vec` de-duplicates only the right-hand side, so the law is stated over set-shaped
        // rows (which is what the store writes).
        first.known_names = vec![other_name];
        first.source_identities.push(left);
        let mut second = base;
        second.sports = vec![other_sport];
        second.source_identities.push(right);

        let mut merged = first.clone();
        merged.merge(second.clone());
        let mut reversed = second;
        reversed.merge(first);

        prop_assert!(same_members(&merged.known_names, &reversed.known_names));
        prop_assert!(same_members(&merged.sports, &reversed.sports));
        prop_assert!(same_members(&merged.source_identities, &reversed.source_identities));
    }

    #[test]
    fn event_unions_are_commutative(
        base in event(),
        left in word(20),
        right in word(20),
        evidence in evidence(),
    ) {
        let mut first = base.clone();
        first.source_labels.push(SourceEventLabel {
            source: SourceRef::new("wiaa_results", None),
            label: left,
        });
        let mut second = base;
        second.source_labels.push(SourceEventLabel {
            source: SourceRef::new("raceday", None),
            label: right,
        });
        second.evidence.push(evidence);

        let mut merged = first.clone();
        merged.merge(second.clone());
        let mut reversed = second;
        reversed.merge(first);

        prop_assert!(same_members(&merged.source_labels, &reversed.source_labels));
        prop_assert!(same_members(&merged.evidence, &reversed.evidence));
    }
}
