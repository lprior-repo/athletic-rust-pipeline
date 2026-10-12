use crate::school_directory::{
    CityName, DirectoryIndex, IdentifiedKey, LinkDecision, NcesSchoolId, PostalAddress,
    SchoolDirectoryEntry, SchoolName, SourceLabel, StreetLine,
};
use crate::UsJurisdiction;

#[test]
fn nces_without_city_corroboration_never_links() -> super::TestResult {
    let address = PostalAddress::of(
        Some(StreetLine::parse("1 Main St")?),
        None,
        Some(CityName::parse("Springfield")?),
        Some(UsJurisdiction::Ohio),
        None,
    )
    .ok_or("missing address")?;
    let entry = SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("390000000001")?),
        SourceLabel::Ccd,
        Some(SchoolName::parse("Springfield High School")?),
    )
    .with_address(Some(address));
    let index = DirectoryIndex::build(&[entry]);
    for city in [None, Some("Columbus")] {
        let decision = index.link_name("Springfield High School", city, UsJurisdiction::Ohio, &[]);
        println!("city={city:?} decision={decision:?}");
        check!(eq; decision, LinkDecision::NoMatch);
    }
    Ok(())
}
