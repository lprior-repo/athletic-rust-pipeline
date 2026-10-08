use super::SEASON;
use anyhow::{Context, Result};
use census_crawl::{compiled, hytek};
use census_domain::model::SourceRef;

const REGIONAL_EXPORT: &str = r#"
05/26/2026, 09:56 PM                                  D1 Regional 8B - Appleton North
                                                        Appleton North HS  Tue, May 26, 2026
                                                                     Results
Girls' 4x800 Relay Division 1                     Finals                    Girls' 100 Meters Division 1              Prelims
       Team                    Relay        Finals             Pts               Athlete                 Yr Team              Prelims
1      HORTONVILLE             'A'          9:55.11            10           1    Parrish, Ashley         11   APPLETON NOR…   12.30 Q
    1) Wloszczynski, Lexi 10         2) Young, Ellie 9                      2    Thompson, Emily         12   APPLETON NOR…   12.74 q
    3) Falbo, Hailey 12              4) Huza, Hannah 12                     3    Rades, Jayla            11   HORTONVILLE     12.83 Q
                                                                            4    Rezash, Johannah        12   WEST DE PERE    13.12 q
2      APPLETON NORTH          'A'          9:56.50            8
                                                                            5    Lopez, Eilianyz         10   WEST DE PERE    13.38 q
    1) Dehlinger, Audry 11           2) Brazzale, Elise 10                  6    Hammen, Allie           9    APPLETON WEST   13.39 q
    3) Busch, Sophia 12              4) Helmbrecht, Ava 12
                                                                            7    Josephson, Sydney       9    KAUKAUNA        13.44 q
3      KIMBERLY                'A'          10:03.38           6            8    Olson, Denise           12    APPLETON EAST   13.55 q
"#;

const PAGE_STAMP_EXPORT: &str = "5/27/25, 8:35 PM                                              Manage D3 Regional 4B - Deerfield\n\
                          D3 Regional 4B - Deerfield\n\
                     Deerfield HS Track   Tue, May 27, 2025\n\
                                  Results\n";

const HEADERLESS_EXPORT: &str = "Girls' 100 Meters Division 1   Finals";

#[test]
fn compiled_documented_exports_preserve_published_meet_and_event_blocks() -> Result<()> {
    let source = SourceRef::new("wiaa_results", None);

    let regional = compiled::parse(
        &hytek::lines_from_pdf_text(REGIONAL_EXPORT),
        source.clone(),
        SEASON,
    );
    let regional = regional.context("the regional export has a meet header")?;
    anyhow::ensure!(
        regional.name == "D1 Regional 8B - Appleton North",
        "left={:?} right={:?}",
        &regional.name,
        &"D1 Regional 8B - Appleton North"
    );
    anyhow::ensure!(
        regional.date == "2026-05-26",
        "left={:?} right={:?}",
        &regional.date,
        &"2026-05-26"
    );
    anyhow::ensure!(
        regional.events.len() == 2,
        "one event per block — left={:?} right={:?}",
        &regional.events.len(),
        &2
    );

    let page_stamp = compiled::parse(
        &hytek::lines_from_pdf_text(PAGE_STAMP_EXPORT),
        source.clone(),
        SEASON,
    );
    anyhow::ensure!(
        page_stamp.is_none(),
        "a dated page stamp without event rows is not a meet"
    );

    let headerless = compiled::parse(
        &hytek::lines_from_pdf_text(HEADERLESS_EXPORT),
        source,
        SEASON,
    );
    anyhow::ensure!(
        headerless.is_none(),
        "an event heading without a meet header is not a meet"
    );
    Ok(())
}
