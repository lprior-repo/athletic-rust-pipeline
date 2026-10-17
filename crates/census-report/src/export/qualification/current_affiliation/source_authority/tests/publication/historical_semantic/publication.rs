use super::*;

#[test]
fn controlled_forged_v2_current_selected_admission_preserves_prior_valid_publication() -> TestResult
{
    let (valid, expected) = setup::prepare(Case::Resolved)?;
    let objects = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let dataset = capture(&valid, objects.path())?;
    witness(&dataset, &expected)?;
    let root = output.path().join("publication");
    let workbook = publish(&valid, &dataset, &root)?;
    artifacts::readback(&workbook, &dataset, &expected)?;
    let pointer = std::fs::read_link(root.join("current"))?;
    let generation = workbook.parent().ok_or("valid publication generation")?;
    let generation_digest = published::verify_published(&workbook)?.generation_digest;
    let retained = [
        "manifest.json",
        "frozen-input.json",
        "athletes.csv",
        "recruiting.csv",
        "current_affiliations.csv",
    ]
    .into_iter()
    .map(|name| Ok((name, std::fs::read(generation.join(name))?)))
    .collect::<TestResult<Vec<_>>>()?;
    let workbook_bytes = std::fs::read(&workbook)?;
    let hostile = fixture::Fixture::new()?;
    let source = fixture::install(&hostile, "12439", "42", Sport::IndoorTrack, true)?;
    hostile.accept(&source.athlete.id)?;
    forged_issuance::review_forged(
        &hostile,
        &source,
        "controlled-forged-report-admission",
        Mutation::OmitMember,
    )?;
    let hostile_objects = tempfile::tempdir()?;
    refuse_bound_historical(&hostile, hostile_objects.path())?;
    let refused_archive = output.path().join("refused.json");
    match admit_save_publish(&hostile, hostile_objects.path(), &refused_archive, &root) {
        Err(crate::report::ReportError::Store(census_store::StoreError::Refused { detail })) => {
            check!(eq; detail, "roster source context object reference mismatch");
        }
        _ => {
            return Err(
                "forged current selected admission must refuse exact interpretation reference"
                    .into(),
            )
        }
    }
    check!(!refused_archive.exists());
    check!(eq; std::fs::read_link(root.join("current"))?, pointer);
    check!(eq; published::verify_published(&workbook)?.generation_digest, generation_digest);
    check!(eq; std::fs::read(&workbook)?, workbook_bytes);
    for (name, bytes) in retained {
        check!(eq; std::fs::read(generation.join(name))?, bytes);
    }
    check!(eq; publish(&valid, &dataset, &root)?, workbook);
    artifacts::readback(&workbook, &dataset, &expected)?;
    check!(eq; published::verify_published(&workbook)?.generation_digest, generation_digest);
    Ok(())
}

fn admit_save_publish(
    fixture: &fixture::Fixture,
    objects: &Path,
    archive: &Path,
    root: &Path,
) -> crate::report::ReportResult<PathBuf> {
    let selection = ProfileSelectionV1::new(fixture.store.store_identity()?, fixture.profile.id()?);
    let dataset = ExportDataset::from_selected_snapshot(
        &fixture.store,
        &fixture.store.snapshot(),
        &selection,
        objects,
    )?;
    dataset.save_frozen(archive)?;
    let options = Options {
        grad_year: Some(2027),
        out: Some(root.to_path_buf()),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: fixture.profile.input().jurisdictions.clone(),
        school_year: fixture.profile.input().contact_school_year,
    };
    let censuses = Censuses::of(&dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    workbook::build_from(&dataset, &fixture.store, &options, &censuses)
}

fn refuse_bound_historical(fixture: &fixture::Fixture, objects: &Path) -> TestResult {
    let dataset = fixture.dataset()?;
    let snapshot = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let claims = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    let athletes = crate::export::qualification::physical_observations::capture::<CanonicalAthlete>(
        &dataset,
        &snapshot,
        Table::Athletes,
        objects,
        fixture::budget(),
    )?;
    let bound = BoundRosterSourceViewV1::capture(
        &dataset,
        &fixture.profile,
        objects,
        &claims,
        athletes,
        fixture::instant()?,
        &mut grant,
    )?;
    match crate::export::qualification::current_affiliation::source_authority::assess_historical(
        &dataset,
        &fixture.profile,
        &bound,
        &mut grant,
    ) {
        Err(crate::report::ReportError::Store(census_store::StoreError::Refused { detail })) => {
            check!(eq; detail, "retained semantic inventory differs from original raw V1 interpretation");
        }
        _ => {
            return Err("bound historical Report consumer must refuse forged raw semantics".into())
        }
    }
    check!(grant.ensure_available().is_err());
    Ok(())
}
