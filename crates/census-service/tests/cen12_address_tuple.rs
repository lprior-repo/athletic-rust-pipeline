#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

#[path = "cen12_address_tuple/helpers.rs"]
mod helpers;

use census_crawl::nces::{parse_ccd, parse_pss};
use census_domain::model::{CanonicalSchool, SchoolYear};
use census_domain::school_directory::{AddressKind, PostalAddress, SchoolDirectoryEntry};
use census_domain::UsJurisdiction;
use census_report::report::Scope;
use census_service::school_address::{self, join_generation, Mode, Overrides};
use census_store::{Store, Table};
use helpers::{args, seed, verify_corpus_csv, verify_retained_notes, verify_workbook};
use std::collections::BTreeMap;

const CCD: &str = include_str!("../../census-crawl/tests/fixtures/cen12/ccd_tuples.csv");
const PSS: &str = include_str!("../../census-crawl/tests/fixtures/cen12/pss_mailing.csv");
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn components(address: &PostalAddress) -> [String; 6] {
    [
        address
            .line1()
            .map_or_else(String::new, |value| value.as_str().to_owned()),
        address
            .line2()
            .map_or_else(String::new, |value| value.as_str().to_owned()),
        address
            .city()
            .map_or_else(String::new, |value| value.as_str().to_owned()),
        address
            .state()
            .map_or_else(String::new, |value| value.code().to_owned()),
        address.zip().map_or_else(String::new, ToString::to_string),
        match address.kind() {
            AddressKind::Physical => "physical",
            AddressKind::Mailing => "mailing",
            AddressKind::Unknown => "unknown",
        }
        .to_owned(),
    ]
}

fn expected() -> [[&'static str; 6]; 7] {
    [
        ["10 Campus Road", "", "", "NJ", "07030", "physical"],
        ["PO Box 901", "", "Hoboken", "NJ", "07030-9999", "mailing"],
        ["", "", "Hoboken", "NJ", "", "physical"],
        ["40 Campus Road", "", "Hoboken", "", "07030", "physical"],
        ["50 Campus Road", "", "Hoboken", "NJ", "", "physical"],
        [
            "PO Box 905",
            "Mail Suite",
            "New York",
            "NY",
            "10005-6666",
            "mailing",
        ],
        [
            "70 Campus Road",
            "Building A",
            "Hoboken",
            "NJ",
            "07030-7777",
            "physical",
        ],
    ]
}

#[test]
fn cen12_nces_published_partial_physical_never_borrows_mailing_components() -> TestResult {
    let parsed = parse_ccd(CCD)?;
    check!(eq; parsed.skipped(), &[]);
    check!(eq; parsed.entries().len(), expected().len());
    for (entry, fields) in parsed.entries().iter().zip(expected()) {
        check!(eq; entry.jurisdiction(), Some(UsJurisdiction::NewJersey));
        check!(eq; components(entry.address().ok_or("missing declared tuple")?), fields.map(str::to_owned));
    }
    let mailing = parsed
        .notes()
        .iter()
        .find(|note| note.line == 2 && note.field == "unselected mailing address")
        .ok_or("unselected mailing capture not disclosed")?;
    check!(eq; mailing.detail, "retained in source capture: street=\"PO Box 900\"; line2=\"Mail Suite\"; city=\"New York\"; state=\"NY\"; zip=\"10001\"; plus4=\"1234\"");
    let invalid = parsed
        .notes()
        .iter()
        .find(|note| note.line == 6 && note.field == "address zip")
        .ok_or("denied published ZIP was lost")?;
    check!(invalid.detail.contains("BAD"));
    Ok(())
}

#[test]
fn cen12_nces_pss_published_mailbox_remains_mailing() -> TestResult {
    let parsed = parse_pss(PSS)?;
    let address = parsed
        .entries()
        .first()
        .and_then(SchoolDirectoryEntry::address)
        .ok_or("missing PSS mailing tuple")?;
    check!(eq; components(address), ["PO Box 12", "", "Hoboken", "NJ", "07030-0012", "mailing"].map(str::to_owned));
    Ok(())
}

#[test]
fn cen12_nces_unusable_published_physical_extension_is_not_fake_absence() -> TestResult {
    let header = CCD.lines().next().ok_or("missing fixture header")?;
    let input = format!("{header}\n340000000099,Extension High School,NJ,,,,,,0044,PO Box 99,,Hoboken,NJ,07030,9999,,09,12,No\n");
    let parsed = parse_ccd(&input)?;
    let entry = parsed
        .entries()
        .first()
        .ok_or("school with denied tuple disappeared")?;
    check!(eq; entry.address(), None);
    check!(eq; entry.jurisdiction(), Some(UsJurisdiction::NewJersey));
    let denied = parsed
        .notes()
        .iter()
        .find(|note| note.field == "address zip")
        .ok_or("published extension diagnosis disappeared")?;
    check!(eq; denied.detail, "extension \"0044\" without base ZIP; retained in source capture");
    let unselected = parsed
        .notes()
        .iter()
        .find(|note| note.field == "unselected mailing address")
        .ok_or("unselected complete mailing claim disappeared")?;
    check!(eq; unselected.detail, "retained in source capture: street=\"PO Box 99\"; line2=\"\"; city=\"Hoboken\"; state=\"NJ\"; zip=\"07030\"; plus4=\"9999\"");
    Ok(())
}

#[test]
fn cen12_nces_parser_join_and_published_workbook_keep_exact_declared_tuples() -> TestResult {
    let dir = tempfile::tempdir()?;
    let generation = dir.path().join("generation");
    school_address::run(&args(&generation))?;
    let parsed = parse_ccd(CCD)?;
    let store = Store::open(dir.path().join("store"))?;
    seed(&store, parsed.entries())?;
    let overrides = Overrides {
        urls: BTreeMap::from([(
            "nces-ccd".into(),
            "https://nces.ed.gov/ccd/tuple-regression.csv".into(),
        )]),
        dates: BTreeMap::from([("nces-ccd".into(), "2026-10-07".into())]),
    };
    let joined = join_generation(&store, &generation, None, overrides, Mode::Apply)?;
    check!(eq; joined.counters.linked, 7);
    check!(eq; joined.counters.refused, 0);
    let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
    for entry in parsed.entries() {
        let school = schools
            .iter()
            .find(|school| Some(school.name.as_str()) == entry.name().map(|name| name.as_str()))
            .ok_or("joined school absent")?;
        let claim = school
            .postal_addresses
            .first()
            .ok_or("partial source tuple not joined")?;
        check!(eq; components(claim.address()), components(entry.address().ok_or("source tuple absent")?));
        check!(eq; claim.owner().namespace, census_domain::model::SourceNamespace::school_directory("nces-ccd", UsJurisdiction::NewJersey));
    }
    let path = census_report::workbook::build(
        &store,
        &census_report::workbook::Options {
            grad_year: None,
            out: Some(dir.path().join("book.xlsx")),
            limit: None,
            scope: Scope::AllSources,
            school_year: SchoolYear::new(2025).ok_or("bad fixture year")?,
        },
    )?;
    verify_workbook(&path, parsed.entries())?;
    verify_corpus_csv(&generation)?;
    verify_retained_notes(&generation)?;
    Ok(())
}
