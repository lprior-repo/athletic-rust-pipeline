//! The coach contact policy, applied by `Entity::publish`.
//!
//! `publish` is what every read of the store passes through, so the collection contract — a
//! published mailbox ships only when it is not a consumer address — has to hold for the merged row,
//! not for the observation that happened to carry the address. The address policy itself is not
//! restated here: `census_domain::model::professional_email` is the oracle, and these properties
//! pin `publish` to it.

use super::*;

proptest! {
    #![proptest_config(law_config())]

    #[test]
    fn publish_applies_the_shared_contact_policy(address in mailbox()) {
        let mut coach = coach_with_email(address.clone());
        let published = census_domain::model::professional_email(&address);

        coach.publish();
        prop_assert_eq!(&coach.professional_email, &published);
        prop_assert_eq!(coach.email_withheld, published.is_none());
        prop_assert_eq!(coach.withheld_mailboxes(), usize::from(coach.email_withheld));

        // Publishing is idempotent: a re-read of the same row cannot withhold more than once.
        let once = coach.clone();
        coach.publish();
        prop_assert_eq!(coach, once);
    }

    #[test]
    fn publish_targets_the_address_the_first_writer_contributed(
        first in mailbox(),
        second in mailbox(),
    ) {
        let mut merged = coach_with_email(first.clone());
        merged.merge(coach_with_email(second));
        prop_assert_eq!(&merged.professional_email, &Some(first.clone()));

        merged.publish();
        prop_assert_eq!(
            &merged.professional_email,
            &census_domain::model::professional_email(&first)
        );
        prop_assert_eq!(merged.withheld_mailboxes(), usize::from(merged.email_withheld));
    }

    #[test]
    fn a_shipped_address_is_one_the_policy_accepts(address in mailbox()) {
        let mut coach = coach_with_email(address);
        coach.publish();
        match coach.professional_email.as_deref() {
            Some(published) => {
                prop_assert!(!coach.email_withheld, "a shipped address is not withheld");
                prop_assert_eq!(
                    census_domain::model::professional_email(published),
                    Some(published.to_string()),
                    "the shipped address survives the policy: {}", published
                );
            }
            None => prop_assert!(coach.email_withheld, "dropping an address is recorded"),
        }
    }
}
