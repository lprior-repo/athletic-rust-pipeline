use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn varsity_output_survives_rejected_duplicates_in_either_order() -> TestResult {
    for levels in [["Varsity", "JV"], ["JV", "Varsity"]] {
        let teams: Vec<_> = levels
            .iter()
            .map(|level| {
                serde_json::json!({
                    "name": "Boys' Cross Country", "level": level, "coachProfileIds": ["staff-1"]
                })
            })
            .collect();
        let summary: SchoolSummary = serde_json::from_value(serde_json::json!({
            "name": "Test High School", "shortCode": "TEST",
            "staff": [{"id": "staff-1", "firstName": "Ada", "lastName": "Lovelace", "title": "Head Coach"}],
            "teams": teams
        }))?;
        let (_, school_id) = minted(UsJurisdiction::Wyoming, "Test High School");
        let emission = emitted(&summary, &school_id, "https://example.test/summary")?;
        let rows: Vec<_> = emission
            .coaches
            .iter()
            .map(|coach| (coach.name.as_str(), coach.sport, coach.gender, coach.role))
            .collect();
        check!(eq;
            rows,
            [(
                "Ada Lovelace",
                Some(Sport::CrossCountry),
                Gender::Boys,
                CoachRole::HeadCoach
            )]
        );
        check!(eq; emission.counters.dropped_levels.get("JV"), Some(&1));
    }
    Ok(())
}

#[test]
fn rejected_vendor_contact_cannot_claim_a_valid_persons_row() -> TestResult {
    let summary: SchoolSummary = serde_json::from_value(serde_json::json!({
        "name": "Test High School", "shortCode": "TEST",
        "staff": [
            {"id": "vendor", "firstName": "Ada", "lastName": "Lovelace", "title": "Head Coach", "emails": ["support@dragonflyathletics.com"]},
            {"id": "real", "firstName": "Ada", "lastName": "Lovelace", "title": "Head Coach", "emails": ["ada@school.edu"]}
        ],
        "teams": [{"name": "Boys' Cross Country", "level": "Varsity", "coachProfileIds": ["vendor", "real"]}]
    }))?;
    let (_, school_id) = minted(UsJurisdiction::Wyoming, "Test High School");
    let emission = emitted(&summary, &school_id, "https://example.test/summary")?;
    let rows: Vec<_> = emission
        .coaches
        .iter()
        .map(|coach| (coach.name.as_str(), coach.professional_email.as_deref()))
        .collect();
    check!(eq; rows, [("Ada Lovelace", Some("ada@school.edu"))]);
    check!(eq; emission.counters.dropped_vendor, 1);
    Ok(())
}
