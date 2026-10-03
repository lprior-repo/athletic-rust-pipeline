use super::{parse_school_list, parse_school_page};
use census_domain::model::{CoachRole, Gender, Sport};
use census_domain::school_directory::StateRecordId;

pub(super) const RETAINED: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/tssaa/directory_id157.html"
));
pub(super) const CAPTURED: &str = "2026-09-22T16:10:49Z";

#[test]
fn retained_page_preserves_exact_school_postal_address_and_appointments() -> anyhow::Result<()> {
    let text = std::str::from_utf8(RETAINED)?;
    let read = parse_school_page(text, &StateRecordId::parse("157")?)?;
    let entry = read
        .school
        .entries()
        .first()
        .ok_or_else(|| anyhow::anyhow!("school"))?;
    check!(eq;
        entry.name().map(|name| name.as_str()),
        Some("Page High School")
    );
    check!(eq; entry.key().label(), "state:TN:157");
    let address = entry.address().ok_or_else(|| anyhow::anyhow!("address"))?;
    check!(eq;
        address.line1().map(|line| line.as_str()),
        Some("6281 Arno Rd")
    );
    check!(eq; address.city().map(|city| city.as_str()), Some("Franklin"));
    check!(eq; address.zip().map(|zip| zip.code()), Some("37064"));
    let ad = read
        .coaches
        .iter()
        .find(|coach| coach.person == "Benji Gray" && coach.sport.is_none())
        .ok_or_else(|| anyhow::anyhow!("published AD"))?;
    check!(eq; ad.person, "Benji Gray");
    check!(eq;
        (ad.sport, ad.gender, ad.role),
        (None, Gender::Unknown, CoachRole::AthleticDirector)
    );
    check!(eq; ad.email.as_deref(), Some("benjaming@wcs.edu"));
    check!(eq; ad.phone.as_deref(), Some("865-405-1056"));
    assert_retained_sport_appointments(&read)?;
    Ok(())
}

fn assert_retained_sport_appointments(read: &super::SchoolRead) -> anyhow::Result<()> {
    [Gender::Boys, Gender::Girls]
        .into_iter()
        .try_for_each(|gender| {
            assert_sport(
                read,
                Sport::CrossCountry,
                gender,
                &[
                    (
                        "Ron Brock",
                        CoachRole::HeadCoach,
                        Some("ronald.brock@wcs.edu"),
                    ),
                    (
                        "Ralph Ringstaff",
                        CoachRole::AssistantCoach,
                        Some("ralph.ringstaff@cityschools.net"),
                    ),
                ],
            )?;
            assert_sport(
                read,
                Sport::OutdoorTrack,
                gender,
                &[
                    (
                        "Marcos Harris",
                        CoachRole::HeadCoach,
                        Some("mharris1074@gmail.com"),
                    ),
                    (
                        "Kevin Lewis",
                        CoachRole::AssistantCoach,
                        Some("kevin.lewis@wcs.edu"),
                    ),
                    (
                        "Adam Neelly",
                        CoachRole::AssistantCoach,
                        Some("adam.neelly@ampf.com"),
                    ),
                ],
            )
        })
}

fn assert_sport(
    read: &super::SchoolRead,
    sport: Sport,
    gender: Gender,
    expected: &[(&str, CoachRole, Option<&str>)],
) -> anyhow::Result<()> {
    let actual: Vec<_> = read
        .coaches
        .iter()
        .filter(|row| row.sport == Some(sport) && row.gender == gender)
        .map(|row| (row.person.as_str(), row.role, row.email.as_deref()))
        .collect();
    check!(eq; actual, expected);
    Ok(())
}

#[test]
fn retained_page_refuses_foreign_requested_school_owner() -> anyhow::Result<()> {
    let text = std::str::from_utf8(RETAINED)?;
    check!(matches!(
        parse_school_page(text, &StateRecordId::parse("3")?),
        Err(crate::CrawlError::DirectoryArtifact { .. })
    ));
    Ok(())
}

#[test]
fn empty_and_invalid_directory_and_school_pages_remain_distinct() -> anyhow::Result<()> {
    check!(matches!(
        parse_school_list(""),
        Err(crate::CrawlError::DirectoryArtifact { .. })
    ));
    check!(eq; parse_school_list("source: [ ],")?.entries(), &[]);
    let invalid = parse_school_list("source: [ {id: 'bad', name: ''}],")?;
    check!(eq; invalid.entries(), &[]);
    check!(eq; invalid.skipped().len(), 2);
    check!(matches!(
        parse_school_page("", &StateRecordId::parse("157")?),
        Err(crate::CrawlError::DirectoryArtifact { .. })
    ));
    Ok(())
}

#[test]
fn published_unknown_role_and_gender_do_not_create_mailboxes_or_participation() -> anyhow::Result<()>
{
    let text = "<h2>Example School</h2><div class=\"card-header\">Track and Field</div>\
        <tr class=\"staffPerson\"><td>Pat Lee</td><td>Volunteer</td><td>HasEmail: true</td><td></td></tr>\
        <div class=\"card-header\">Boys' Cross Country</div><p>Member of Division I</p>\
        source: [ {id: '1', name: 'Example School (Town, TN)'}],";
    let read = parse_school_page(text, &StateRecordId::parse("1")?)?;
    check!(eq; read.coaches.len(), 1);
    let row = read
        .coaches
        .first()
        .ok_or_else(|| anyhow::anyhow!("appointment"))?;
    check!(eq;
        (row.person.as_str(), row.role, row.gender, row.sport, row.email.as_deref()),
        (
            "Pat Lee",
            CoachRole::Unknown,
            Gender::Unknown,
            Some(Sport::OutdoorTrack),
            None
        )
    );
    check!(eq; row.published_role, "Volunteer");
    Ok(())
}

mod collect;
