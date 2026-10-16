use super::*;
use census_store::Receipt;

pub(in super::super) struct Installed {
    pub(in super::super) candidate: AthleteCandidateId,
    pub(in super::super) athlete: CanonicalAthlete,
    school: CanonicalSchool,
    school_reference: ProviderSchoolPacketRefV1,
    school_payload: Value,
    target_reference: SourceTargetPacketRefV1,
    target_payload: Value,
    receipts: [Receipt; 2],
    source_tables_digest: String,
}

pub(in super::super) fn install(
) -> TestResult<(tempfile::TempDir, Store, PublicIdentityWriterV1, Installed)> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    let school_packet = school_fixtures::packet(school_fixtures::DIRECTORY)?;
    let checked = school_packet.read()?;
    school_fixtures::verify_checked_school(&checked)?;
    let school_recording = Recording::new();
    let before = protected_state(&store)?;
    check!(eq; record_provider_school_intake(&store, &school_recording, &school_packet)?, checked.school().id);
    check!(eq; protected_state(&store)?, before);
    let school = school_recording
        .drain()
        .prepare("accepted-provider-school")?
        .apply(&store)?;
    check!(eq; school.appended(), 2);
    let target = provider_packet(RESULTS)?;
    let target_recording = Recording::new();
    let before = protected_state(&store)?;
    let ProviderTargetIntakeDispositionV1::ProjectedUnaccepted { candidate } =
        record_provider_target_intake(&store, &target_recording, &target, school_budget())?
    else {
        return Err("genuine installed school must resolve provider target".into());
    };
    check!(eq; protected_state(&store)?, before);
    let projected = target_recording
        .drain()
        .prepare("accepted-provider-target")?
        .apply(&store)?;
    check!(eq; projected.appended(), 4);
    no_public_birth(&store)?;
    let athletes = store.snapshot().scan::<CanonicalAthlete>(Table::Athletes)?;
    let [athlete] = athletes.as_slice() else {
        return Err("exactly one genuine projected athlete required".into());
    };
    check!(eq; athlete.id, candidate);
    let installed = Installed {
        candidate: candidate.cast(),
        athlete: athlete.clone(),
        school: checked.school().clone(),
        school_reference: school_packet.reference()?,
        school_payload: serde_json::to_value(&school_packet)?,
        target_reference: target.reference()?,
        target_payload: serde_json::to_value(&target)?,
        receipts: [school.receipt().clone(), projected.receipt().clone()],
        source_tables_digest: source_tables_digest(&store)?,
    };
    installed.verify(&store)?;
    Ok((directory, store, writer, installed))
}

impl Installed {
    pub(in super::super) fn verify(&self, store: &Store) -> TestResult {
        let snapshot = store.snapshot();
        check!(eq; snapshot.scan::<CanonicalSchool>(Table::Schools)?, vec![self.school.clone()]);
        check!(eq; snapshot.scan::<CanonicalAthlete>(Table::Athletes)?, vec![self.athlete.clone()]);
        check!(eq; self.athlete.school, self.school.id);
        check!(eq; self.athlete.id.as_str(), self.candidate.as_str());
        check!(eq; self.athlete.canonical_name, "Zacchaeus Brocks");
        check!(eq; self.athlete.grad_year.get(), 2026);
        check!(eq; self.athlete.gender, Gender::Boys);
        check!(eq; self.athlete.identity_in(&SourceNamespace::MilesplitAthlete)
            .ok_or("genuine provider athlete identity")?.id, "7223777");
        check!(self.athlete.observed_grades.is_empty());
        scoped_observations::verify(&snapshot, self.target_reference.as_str())?;
        check!(eq; source_tables_digest(store)?, self.source_tables_digest);
        for receipt in &self.receipts {
            check!(eq; store.receipt(&receipt.operation)?, Some(receipt.clone()));
        }
        self.verify_roots(&snapshot)
    }

    fn verify_roots(&self, snapshot: &StoreSnapshot<'_>) -> TestResult {
        let mut grant = AttachmentReplayGrantV1::new();
        let school = read_recorded_provider_school(snapshot, &self.school_reference, &mut grant)?
            .ok_or("retained genuine school after acceptance")?;
        let target = read_recorded_provider_target(snapshot, &self.target_reference, &mut grant)?
            .ok_or("retained genuine target after acceptance")?;
        school_fixtures::verify_checked_school(school.checked())?;
        check!(eq; school.checked().school(), &self.school);
        for (root, schema, key, payload) in [
            (
                school.root(),
                RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
                self.school_reference.as_str(),
                &self.school_payload,
            ),
            (
                target.root(),
                RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
                self.target_reference.as_str(),
                &self.target_payload,
            ),
        ] {
            exact_root(snapshot, root, schema, schema.phase(), key, payload)?;
        }
        exact_capture(
            &self.school_payload,
            Some("directory"),
            school_fixtures::DIRECTORY,
        )?;
        exact_capture(
            &self.school_payload,
            Some("roster"),
            school_fixtures::ROSTER,
        )?;
        exact_capture(&self.target_payload, Some("profile"), PROFILE)?;
        exact_capture(&self.target_payload, Some("linked_results"), RESULTS)?;
        Ok(())
    }
}

fn source_tables_digest(store: &Store) -> TestResult<String> {
    let tables = Table::ALL
        .into_iter()
        .filter(|table| *table != Table::AthleteIdentityDecisions)
        .collect::<Vec<_>>();
    Ok(store.snapshot().tables_digest(&tables)?)
}
