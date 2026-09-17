use crate::native_fixture::scenarios::ScenarioSet;
use anyhow::{Context, Result};
use rust_xlsxwriter::Workbook;
use std::{
    fs,
    path::{Path, PathBuf},
};

const HEADERS: [&str; 15] = [
    "Person First",
    "Person Last",
    "Person Email",
    "Address Mailing / Permanent Street Combined",
    "Address Mailing / Permanent City",
    "Address Mailing / Permanent Region",
    "Address Mailing / Permanent Postal",
    "Sports Created Date",
    "Sports Sport",
    "Sports Rating",
    "Origin Source Date",
    "Origin Source",
    "Schools Name",
    "Class Year",
    "Fixture Scenario",
];
#[derive(Debug, Clone)]
struct Row {
    first: &'static str,
    last: &'static str,
    city: &'static str,
    region: &'static str,
    school: &'static str,
    sport: &'static str,
    year: &'static str,
    scenario: &'static str,
}
const ROWS: [Row; 28] = [
    Row {
        first: "Ada",
        last: "Runner",
        city: "Austin",
        region: "TX",
        school: "Central High School",
        sport: "Track & Field",
        year: "Junior",
        scenario: "match",
    },
    Row {
        first: "Ada",
        last: "Runner",
        city: "Austin",
        region: "TX",
        school: "Central High School",
        sport: "Cross Country",
        year: "Junior",
        scenario: "match",
    },
    Row {
        first: "Casey",
        last: "Copy",
        city: "Denver",
        region: "CO",
        school: "Duplicate High",
        sport: "Cross Country",
        year: "Junior",
        scenario: "duplicate",
    },
    Row {
        first: "Casey",
        last: "Copy",
        city: "Denver",
        region: "CO",
        school: "Duplicate High",
        sport: "Cross Country",
        year: "Junior",
        scenario: "duplicate",
    },
    Row {
        first: "Sam",
        last: "Same",
        city: "Boston",
        region: "MA",
        school: "Twin High",
        sport: "Track & Field",
        year: "Junior",
        scenario: "ambiguous",
    },
    Row {
        first: "Morgan",
        last: "Missing",
        city: "Reno",
        region: "NV",
        school: "No Class High",
        sport: "Track & Field",
        year: "",
        scenario: "missing-cohort",
    },
    Row {
        first: "Taylor",
        last: "Clash",
        city: "Portland",
        region: "OR",
        school: "Conflict High",
        sport: "Cross Country",
        year: "Junior",
        scenario: "conflict",
    },
    Row {
        first: "Empty",
        last: "Complete",
        city: "Miami",
        region: "FL",
        school: "No Hits High",
        sport: "Track & Field",
        year: "Junior",
        scenario: "empty-search",
    },
    Row {
        first: "Broken",
        last: "Response",
        city: "Phoenix",
        region: "AZ",
        school: "Malformed High",
        sport: "Track & Field",
        year: "Junior",
        scenario: "malformed",
    },
    Row {
        first: "Retry",
        last: "Exhaust",
        city: "Chicago",
        region: "IL",
        school: "Retry High",
        sport: "Track & Field",
        year: "Junior",
        scenario: "retry-exhaustion",
    },
    Row {
        first: "Huge",
        last: "Payload",
        city: "Seattle",
        region: "WA",
        school: "Payload High",
        sport: "Track & Field",
        year: "Junior",
        scenario: "payload-limit",
    },
    Row {
        first: "Isolated",
        last: "Denied",
        city: "Dallas",
        region: "TX",
        school: "Access High",
        sport: "Track & Field",
        year: "Junior",
        scenario: "access-denied",
    },
    Row {
        first: "Riley",
        last: "Split",
        city: "Austin",
        region: "TX",
        school: "Central High School",
        sport: "Track & Field",
        year: "",
        scenario: "split-location",
    },
    Row {
        first: "Gene",
        last: "Generic",
        city: "Austin",
        region: "TX",
        school: "High School",
        sport: "Track & Field",
        year: "",
        scenario: "generic-school",
    },
    identity_row("Alex", "Coverage", "name-exclusion"),
    identity_row("Álex", "Coverage", "name-exclusion"),
    identity_row("Bert", "Other", "name-exclusion"),
    identity_row("Nora", "Exclusion", "name-exclusion"),
    identity_row("Alex", "Cover'age", "name-exclusion"),
    identity_row("Rae", "Failurecase", "probe-failure"),
    identity_row("Pat", "Bioconflict", "bio-identity-conflict"),
    identity_row("Pat", "Statecase", "html-identity-unknown"),
    identity_row("Pat", "Componentcase", "incomplete-identity"),
    identity_row("Pat", "Bindingcase", "wrong-bio-id"),
    identity_row("Lena", "Displaycase", "misleading-search-name"),
    identity_row("Pat", "Nohintcase", "missing-html-hint"),
    identity_row("Pat", "Rawcase", "raw-identity-conflict"),
    identity_row("Pat", "Htmlaliascase", "html-alias-conflict"),
];

const fn identity_row(first: &'static str, last: &'static str, scenario: &'static str) -> Row {
    Row {
        first,
        last,
        city: "Austin",
        region: "TX",
        school: "Central High School",
        sport: "Track & Field",
        year: "",
        scenario,
    }
}

pub fn generate(output_dir: &Path, selected: &ScenarioSet) -> Result<PathBuf> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("creating fixture output directory {}", output_dir.display()))?;
    let destination = output_dir.join("native-fixture.xlsx");
    if destination.exists() {
        fs::remove_file(&destination)
            .with_context(|| format!("replacing {}", destination.display()))?;
    }
    let mut workbook = Workbook::new();
    write_sheet(
        &mut workbook,
        "Synthetic Roster A",
        ROWS[..6]
            .iter()
            .filter(|row| {
                selected
                    .scenarios
                    .iter()
                    .any(|case| case.as_str() == row.scenario)
            })
            .collect(),
    )?;
    write_sheet(
        &mut workbook,
        "Synthetic Roster B",
        ROWS[6..]
            .iter()
            .filter(|row| {
                selected
                    .scenarios
                    .iter()
                    .any(|case| case.as_str() == row.scenario)
            })
            .collect(),
    )?;
    workbook
        .save(&destination)
        .with_context(|| format!("writing {}", destination.display()))?;
    Ok(destination)
}
fn write_sheet(workbook: &mut Workbook, name: &str, rows: Vec<&Row>) -> Result<()> {
    let worksheet = workbook.add_worksheet().set_name(name)?;
    HEADERS
        .iter()
        .enumerate()
        .try_for_each(|(column, header)| {
            let column = u16::try_from(column).context("fixture header column overflow")?;
            worksheet
                .write_string(0, column, *header)
                .map(|_| ())
                .context("writing fixture header")
        })?;
    rows.iter().enumerate().try_for_each(|(offset, row)| {
        let values = [
            row.first,
            row.last,
            "synthetic@example.invalid",
            "100 Fictional Way",
            row.city,
            row.region,
            "00000",
            "2026-01-01",
            row.sport,
            "0",
            "2026-01-01",
            "synthetic-fixture",
            row.school,
            row.year,
            row.scenario,
        ];
        values.iter().enumerate().try_for_each(|(column, value)| {
            let column = u16::try_from(column).context("fixture data column overflow")?;
            let excel_row = u32::try_from(offset + 1).context("fixture row overflow")?;
            worksheet
                .write_string(excel_row, column, *value)
                .map(|_| ())
                .context("writing fixture data")
        })
    })
}
