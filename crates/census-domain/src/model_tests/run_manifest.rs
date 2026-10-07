use super::*;

#[test]
fn a_run_revision_starts_at_one() -> Result<(), Box<dyn std::error::Error>> {
    let season = SchoolYear::new(2026).ok_or("2026 is a school year")?;
    for revision in [1, 2, 17, u32::MAX] {
        check!(eq; CensusRun::new(season, revision).map(CensusRun::revision), Some(revision));
    }
    check!(
        CensusRun::new(season, 0).is_none(),
        "a run cannot carry revision zero"
    );
    check!(eq;
        CensusRun::new(season, 4).map(CensusRun::season),
        Some(season),
        "the run keeps the season it was submitted under"
    );
    Ok(())
}

#[test]
fn a_serialized_run_cannot_carry_a_revision_below_one() -> Result<(), Box<dyn std::error::Error>> {
    let season = SchoolYear::new(2026).ok_or("2026 is a school year")?;
    let run = CensusRun::new(season, 3).ok_or("revision 3 is a run revision")?;
    let text = serde_json::to_string(&run)?;
    check!(eq;
        text, "{\"season\":2026,\"revision\":3}",
        "the serialized run is the store's contract"
    );
    check!(eq; serde_json::from_str::<CensusRun>(&text)?, run);

    check!(
        serde_json::from_str::<CensusRun>("{\"season\":2026,\"revision\":0}").is_err(),
        "a stored revision of zero is refused rather than clamped"
    );
    check!(
        serde_json::from_str::<CensusRun>("{\"season\":1800,\"revision\":1}").is_err(),
        "a season outside the school years is refused"
    );
    check!(
        serde_json::from_str::<CensusRun>("{\"season\":2026,\"revision\":1,\"extra\":1}").is_err(),
        "an unknown field in a stored run is refused rather than ignored"
    );
    Ok(())
}

#[test]
fn a_manifest_binds_the_store_the_run_the_cohort_and_the_scope(
) -> Result<(), Box<dyn std::error::Error>> {
    let season = SchoolYear::new(2026).ok_or("2026 is a school year")?;
    let manifest = RunManifest {
        store_identity: "a".repeat(64),
        run: CensusRun::new(season, 1).ok_or("revision 1 is a run revision")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::NewYork, UsJurisdiction::Ohio],
    };
    let text = serde_json::to_string(&manifest)?;
    check!(eq; serde_json::from_str::<RunManifest>(&text)?, manifest);
    check!(
        serde_json::from_str::<RunManifest>(&text.replace("\"NY\"", "\"ZZ\"")).is_err(),
        "a jurisdiction the domain does not know is refused"
    );
    check!(
        serde_json::from_str::<RunManifest>(&text.replace("\"revision\":1", "\"revision\":0"))
            .is_err(),
        "a manifest whose run revision is zero is refused"
    );
    check!(
        serde_json::from_str::<RunManifest>(&text.replace("\"run\"", "\"second_run\"")).is_err(),
        "a manifest that renames its run binding is refused"
    );
    Ok(())
}
