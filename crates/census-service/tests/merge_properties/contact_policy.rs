//! The coach contact policy, applied by `Entity::publish`.
//!
//! `publish` is what every read of the store passes through, so every valid published mailbox must
//! survive on the merged row in the field matching its domain kind. Malformed addresses are the
//! only inputs that do not produce a published field.

use super::*;
use census_domain::model::{published_email, MailboxKind};

proptest! {
    #![proptest_config(law_config())]

    #[test]
    fn publish_routes_valid_addresses_and_rejects_malformed(address in mailbox()) {
        let mut coach = coach_with_email(address.clone());
        let expected = published_email(&address);

        coach.publish();
        let expected_fields = match expected {
            Some((address, MailboxKind::Professional)) => (Some(address), None),
            Some((address, MailboxKind::Personal)) => (None, Some(address)),
            None => (None, None),
        };
        prop_assert_eq!(
            (&coach.professional_email, &coach.personal_email),
            (&expected_fields.0, &expected_fields.1)
        );

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
        let expected = published_email(&first);
        let expected_fields = match expected {
            Some((address, MailboxKind::Professional)) => (Some(address), None),
            Some((address, MailboxKind::Personal)) => (None, Some(address)),
            None => (None, None),
        };
        prop_assert_eq!(
            (&merged.professional_email, &merged.personal_email),
            (&expected_fields.0, &expected_fields.1)
        );
    }
}
