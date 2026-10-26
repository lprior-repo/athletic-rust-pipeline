use super::*;
use crate::participation_evidence::provider_target::profile_pr::read_source_rows;
use census_domain::UsJurisdiction;

const FIXTURE: &[u8] =
    include_bytes!("../../../tests/fixtures/milesplit/profile_14222592_raw.html");

fn target() -> ProfilePrTarget {
    ProfilePrTarget {
        athlete_id: "14222592".to_string(),
        profile_url: "https://al.milesplit.com/athletes/14222592-adelyn-spann".to_string(),
        site: UsJurisdiction::Alabama,
    }
}

fn parsed_fixture() -> Vec<profile_pr::SourceProfilePrV1> {
    let body = String::from_utf8(FIXTURE.to_vec()).expect("fixture is utf-8");
    read_source_rows(
        &body,
        &target().profile_url,
        target().site,
        &target().athlete_id,
    )
    .expect("fixture parses")
    .expect("fixture publishes a PR table")
}

#[test]
fn fixture_table_maps_to_journal_row() {
    let row = journal_row(
        &target(),
        "2026-10-26T00:00:00Z",
        "abc123",
        parsed_fixture(),
    )
    .expect("journal row builds")
    .expect("fixture rows are nonempty");
    assert_eq!(row.athlete_id(), "14222592");
    assert_eq!(row.site(), "AL");
    assert_eq!(
        row.profile_url(),
        "https://al.milesplit.com/athletes/14222592-adelyn-spann"
    );
    assert_eq!(row.body_sha256(), "abc123");
    assert_eq!(row.rows().len(), 8);
    assert!(row.rows().iter().all(|mark| !mark.event().is_empty()));
    assert!(row.rows().iter().all(|mark| !mark.mark().is_empty()));
    let roundtrip: ProfilePrJournalRowV1 =
        serde_json::from_slice(&serde_json::to_vec(&row).expect("row serializes"))
            .expect("row deserializes");
    assert_eq!(roundtrip, row);
}

#[test]
fn empty_table_maps_to_no_row() {
    let row = journal_row(&target(), "2026-10-26T00:00:00Z", "abc123", Vec::new())
        .expect("empty table is not an error");
    assert!(row.is_none());
}

#[test]
fn journal_key_shards_site_and_scopes_athlete() {
    let first = journal_key(&target());
    let second = journal_key(&target());
    assert_eq!(first, second);
    let (shard, rest) = first.split_at(2);
    assert!(shard.chars().all(|lead| lead.is_ascii_hexdigit()));
    assert_eq!(rest, ":AL:14222592");
}

#[test]
fn invalid_timestamp_refuses_journal_row() {
    let parsed = parsed_fixture();
    let refused = journal_row(&target(), "not-a-timestamp", "abc123", parsed);
    assert!(refused.is_err());
}
