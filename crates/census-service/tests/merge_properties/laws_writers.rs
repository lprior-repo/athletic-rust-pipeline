//! First-writer-wins, identity preservation and hole filling.
//!
//! Every scalar a merge *does* replace is a documented hole fill rather than an overwrite: the
//! meet `level` is written only when the row itself says `Unknown`, and the athlete
//! `identity_confidence` is derived from the cohort the observations agree on. Everything else —
//! the names a canonical record was minted from included — belongs to the first writer.

use super::*;
use census_domain::model::NaturalKey;

proptest! {
    #![proptest_config(law_config())]

    #[test]
    fn school_scalar_fields_keep_the_first_observation(
        city in word(12),
        other_city in word(12),
        association in word(8),
        other_association in word(8),
        enrollment in 0u32..4_000,
        other_enrollment in 0u32..4_000,
        name in word(12),
        other_name in word(12),
        co_op in any::<bool>(),
        other_co_op in any::<bool>(),
    ) {
        let (mut first, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Madison", "madison");
        let (mut second, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Madison", "madison");
        first.city = Some(city.clone());
        second.city = Some(other_city.clone());
        first.association = Some(association.clone());
        second.association = Some(other_association);
        first.enrollment = Some(enrollment);
        second.enrollment = Some(other_enrollment);
        first.name = name.clone();
        second.name = other_name;
        first.co_op = co_op;
        second.co_op = other_co_op;
        let normalized = first.normalized_name.clone();

        let mut merged = first.clone();
        merged.merge(second.clone());
        prop_assert_eq!(&merged.city, &Some(city));
        prop_assert_eq!(&merged.association, &Some(association));
        prop_assert_eq!(merged.enrollment, Some(enrollment));
        prop_assert_eq!(&merged.name, &name);
        prop_assert_eq!(&merged.normalized_name, &normalized);
        prop_assert_eq!(merged.co_op, co_op | other_co_op);

        // The reverse merge keeps the other observation's values: the first writer survives, not
        // one fixed side of the merge.
        let mut reversed = second;
        reversed.merge(first);
        prop_assert_eq!(&reversed.city, &Some(other_city));
        prop_assert_eq!(reversed.enrollment, Some(other_enrollment));
        prop_assert_eq!(&reversed.co_op, &(co_op | other_co_op));
    }

    /// A name that extends the kept name as its prefix ("Nicolet" -> "Nicolet High School") is the
    /// more specific legal name and replaces it; a shorter prefix of the kept name never does.
    #[test]
    fn school_name_keeps_the_longer_prefix_extension(
        short in word(8),
        suffix in word(10),
    ) {
        let long_form = format!("{short} {suffix}");
        let (mut first, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Madison", "madison");
        let (mut second, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Madison", "madison");
        first.name = short.clone();
        second.name = long_form.clone();

        let mut extend = first.clone();
        extend.merge(second.clone());
        prop_assert_eq!(&extend.name, &long_form);

        let mut shorten = second;
        shorten.merge(first);
        prop_assert_eq!(&shorten.name, &long_form);
    }

    #[test]
    fn team_level_is_first_writer_wins_and_survives_a_missing_level(
        level in word(10),
        other_level in word(10),
        base in team(),
    ) {
        let mut known = base.clone();
        known.level = Some(level.clone());
        let mut also_known = base.clone();
        also_known.level = Some(other_level.clone());
        let mut missing = base;
        missing.level = None;

        let mut merged = known.clone();
        merged.merge(also_known.clone());
        prop_assert_eq!(&merged.level, &Some(level.clone()));

        let mut reversed = also_known;
        reversed.merge(known.clone());
        prop_assert_eq!(&reversed.level, &Some(other_level));

        let mut filled = known.clone();
        filled.merge(missing.clone());
        prop_assert_eq!(&filled.level, &Some(level.clone()));

        let mut kept = missing;
        kept.merge(known);
        prop_assert_eq!(&kept.level, &Some(level));
    }

    #[test]
    fn meet_level_fills_an_unknown_hole_and_keeps_a_known_one(
        base in meet(),
        level in level(),
        other_level in level(),
    ) {
        let mut first = base.clone();
        first.level = level;
        let mut second = base;
        second.level = other_level;

        let mut merged = first.clone();
        merged.merge(second.clone());
        let expected = if level == CompetitionLevel::Unknown { other_level } else { level };
        prop_assert_eq!(merged.level, expected);

        let mut reversed = second;
        reversed.merge(first);
        let reverse_expected =
            if other_level == CompetitionLevel::Unknown { level } else { other_level };
        prop_assert_eq!(reversed.level, reverse_expected);
    }

    #[test]
    fn meet_location_is_first_writer_wins(base in meet(), location in word(12), other in word(12)) {
        let mut first = base.clone();
        first.location = Some(location.clone());
        let mut second = base;
        second.location = Some(other.clone());

        let mut merged = first.clone();
        merged.merge(second.clone());
        prop_assert_eq!(&merged.location, &Some(location));

        let mut reversed = second;
        reversed.merge(first);
        prop_assert_eq!(&reversed.location, &Some(other));
    }

    /// One subject under two observations: the alias is absorbed and the name the row was minted from
    /// is not, because a name is what the id is derived from and rewriting it would rewrite identity.
    #[test]
    fn athlete_merge_unions_known_names_under_one_natural_key(base in athlete(), alias in word(20)) {
        let mut athlete = base.clone();
        let canonical = athlete.canonical_name.clone();
        let mut incoming = base;
        incoming.known_names = vec![alias.clone()];

        athlete.merge(incoming);
        prop_assert_eq!(&athlete.canonical_name, &canonical);
        prop_assert!(athlete.known_names.contains(&alias));
        prop_assert!(athlete.known_names.contains(&canonical));
        prop_assert!(athlete.retained_conflicts.is_empty());
    }

    /// Two rows that share an id but not a natural key are an id collision, not one subject: the row
    /// already there keeps every field it holds — its name and its known names included — and the
    /// finding is recorded for an operator rather than absorbed into a third subject.
    #[test]
    fn athlete_merge_records_a_second_name_instead_of_absorbing_it(
        base in athlete(),
        other_name in word(20),
    ) {
        let mut incoming = base.clone();
        incoming.canonical_name = other_name.clone();
        incoming.known_names = vec![other_name.clone()];
        prop_assume!(!base.same_natural_key(&incoming));

        let mut athlete = base;
        let canonical = athlete.canonical_name.clone();
        let known_names = athlete.known_names.clone();
        athlete.merge(incoming);
        prop_assert_eq!(&athlete.canonical_name, &canonical);
        prop_assert_eq!(&athlete.known_names, &known_names);
        prop_assert_eq!(athlete.retained_conflicts.len(), 1);
    }

    #[test]
    fn coach_merge_keeps_the_first_contact_details(
        address in mailbox(),
        other_address in mailbox(),
        base in coach(),
    ) {
        let mut first = base.clone();
        first.professional_email = Some(address.clone());
        first.phone = Some("608-555-0100".to_string());
        let mut second = base;
        second.professional_email = Some(other_address.clone());
        second.phone = Some("608-555-0199".to_string());

        // Each side's address reaches the field its own domain names, and the first writer of a
        // field keeps it: the later side can only fill the field the earlier one left empty.
        let address_slots = published_slots(&address);
        let other_slots = published_slots(&other_address);
        let want = merged_slots(&address_slots, &other_slots);
        let reversed_want = merged_slots(&other_slots, &address_slots);

        let mut merged = first.clone();
        merged.merge(second.clone());
        prop_assert_eq!(
            (&merged.professional_email, &merged.personal_email),
            (&want.0, &want.1)
        );
        prop_assert_eq!(merged.phone.as_deref(), Some("608-555-0100"));

        let mut reversed = second;
        reversed.merge(first);
        prop_assert_eq!(
            (&reversed.professional_email, &reversed.personal_email),
            (&reversed_want.0, &reversed_want.1)
        );
        prop_assert_eq!(reversed.phone.as_deref(), Some("608-555-0199"));
    }
}

// ---------------------------------------------------------------------------
// Athlete cohort rule: observed grades drive identity_confidence
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(law_config())]

    #[test]
    fn disagreement_lowers_confidence_and_agreement_raises_it(
        grade in grade(),
        year in school_year(),
        base in athlete(),
    ) {
        let observation = ObservedGrade {
            grade,
            school_year: year,
            source: SourceRef::new("wiaa_results", None),
        };
        let agrees = observation.grad_year() == base.grad_year;
        let mut incoming = base.clone();
        incoming.observed_grades = vec![observation];

        let mut merged = base.clone();
        merged.merge(incoming.clone());
        let expected = if agrees { Confidence::HIGH } else { Confidence::LOW };
        prop_assert_eq!(merged.identity_confidence, expected);

        // A second copy of the same observation cannot move the confidence again.
        let settled = merged.clone();
        merged.merge(incoming);
        prop_assert_eq!(merged, settled);
    }

    #[test]
    fn a_silent_observation_leaves_confidence_alone(confidence in 0u8..=100, base in athlete()) {
        let mut first = base.clone();
        first.identity_confidence = Confidence::new(confidence).expect("0..=100 is a confidence");
        let mut merged = first.clone();
        merged.merge(base);

        prop_assert_eq!(
            merged.identity_confidence,
            Confidence::new(confidence).expect("0..=100 is a confidence")
        );
        prop_assert!(merged.observed_grades.is_empty());
    }
}
