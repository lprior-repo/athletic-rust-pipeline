//! Canonical mapping: parsed CIAC directory rows → canonical entities.
//!
//! The CIAC source publishes no numeric school id; the only join key is the
//! normalized school name (a natural key derived from jurisdiction + name).
//! The source's own school name is recorded as a source identity.
//!
//! Nothing here reads the network, the store or a file.

use super::pages::{parse_gender, parse_sport_label};
use super::{ASSOCIATION, HOST, SOURCE_ID, STATE};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};

/// One parsed school table: a list of (sport_label, coach_name) rows.
#[derive(Debug, Clone, Default)]
pub struct SchoolTable {
    pub rows: Vec<(String, String)>,
}

/// Parsed content from one school's staff table.
#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub school_id: SchoolId,
    pub coaches: Vec<CanonicalCoach>,
}

/// Build canonical school and coach entities from a parsed school table.
///
/// `observed_on` is the date stamped into the evidence row.
pub fn school_entities(school_name: &str, table: &SchoolTable, observed_on: &str) -> SchoolExtract {
    let normalized = normalize_name(school_name);
    let (mut school, school_id) = CanonicalSchool::new(STATE, school_name, &normalized);
    school.association = Some(ASSOCIATION.to_string());
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(format!("{HOST}/Directory.aspx"))),
        observed_on.to_string(),
    ));

    let mut coaches: Vec<CanonicalCoach> = Vec::new();

    for (sport_label, coach_name) in &table.rows {
        if let Some(sport) = parse_sport_label(sport_label) {
            let gender = parse_gender(sport_label);
            if let Some(row) = sport_coach(
                &school_id,
                sport_label,
                coach_name,
                sport,
                gender,
                observed_on,
            ) {
                coaches.push(row);
            }
        }
        // Non-XC/TF rows are parsed but not stored as coach entities;
        // we keep them attributable via the SchoolTable.
    }

    SchoolExtract {
        school,
        school_id,
        coaches,
    }
}

/// One head coach from a staff-table row.
fn sport_coach(
    school_id: &SchoolId,
    sport_label: &str,
    coach_name: &str,
    sport: Sport,
    gender: Gender,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    // A placeholder is the source saying no one holds this row. The parser already drops such rows
    // from the table, but the rule is enforced here too, where the coach is minted: this is the
    // only site that can put a name in the workbook, so it is the one that must refuse.
    if super::pages::is_placeholder_name(coach_name) {
        return None;
    }
    let mut coach = CanonicalCoach::new(
        school_id,
        coach_name.to_string(),
        Some(sport),
        gender,
        CoachRole::HeadCoach,
    );
    coach.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: ASSOCIATION.to_string(),
            },
            format!("coach:{sport_label}"),
        )
        .with_url(format!("{HOST}/Directory.aspx")),
    );
    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(format!("{HOST}/Directory.aspx"))),
        observed_on.to_string(),
    ));
    Some(coach)
}
