use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use census_domain::model::{
    AthleteCandidateId, CanonicalAthlete, Evidence, Gender, GradYear, SchoolId, SourceIdentity,
    SourceNamespace, SourceRef, StoreIdentity,
};
use census_domain::public_identity_history::RegistryHead;
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use census_store::{Store, Table};

use super::{physical, TestResult};

const PRECOMMIT_SEQUENCE: &str = "child-precommit-sequence.bin";

pub(super) const OPERATION: &str = "isolated-public-identity-crash-v1";
pub(super) const CHILD_ROOT: &str = "CENSUS_IDENTITY_CRASH_ROOT";
pub(super) const CHILD_CASE: &str = "CENSUS_IDENTITY_CRASH_CHILD";
pub(super) const CHILD_NAME: &str = "public_identity_crash_child_fixture";

pub(super) struct Fixture {
    pub root: PathBuf,
    pub identity: StoreIdentity,
    pub genesis: RegistryHead,
    pub athlete: CanonicalAthlete,
    pub candidate: AthleteCandidateId,
    pub evidence: u64,
    pub derived: u64,
    pub before: physical::State,
}

impl Fixture {
    pub fn create() -> TestResult<(tempfile::TempDir, Self)> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("identity");
        check!(root.is_absolute());
        let declaration = Store::create_public_identity_qualification_v2(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
        check!(eq; store.store_format()?.lifecycle_revision, Some(6));
        check!(eq; writer.qualification_version(), PublicIdentityQualificationVersion::RetainedAuthorityContextV2);
        let athlete = subject()?;
        store.append(Table::Athletes, &athlete)?;
        let identity = declaration.store_identity().clone();
        let genesis = writer.head(&store)?;
        check!(eq; genesis.sequence, 0);
        let evidence = store.evidence_generation();
        let derived = store.derived_generation();
        store.flush()?;
        drop(writer);
        drop(store);
        let before = physical::read(&root)?;
        Ok((
            directory,
            Self {
                root,
                identity,
                genesis,
                candidate: athlete.id.cast(),
                athlete,
                evidence,
                derived,
                before,
            },
        ))
    }
}

pub(super) fn subject() -> TestResult<CanonicalAthlete> {
    let school = SchoolId::mint("sch", &["isolated-public-identity-crash"]);
    let mut athlete = CanonicalAthlete::new(
        &school,
        "Synthetic Crash Fixture Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "803"),
    )?;
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new(
            "isolated-public-identity-crash",
            Some("https://mi.milesplit.com/athletes/803".to_owned()),
        ),
        "2026-10-14",
    ));
    Ok(athlete)
}

pub(super) fn child() -> TestResult {
    check!(eq; std::env::var(CHILD_CASE)?.as_str(), CHILD_NAME);
    let root = PathBuf::from(std::env::var_os(CHILD_ROOT).ok_or("child root absent")?);
    check!(root.is_absolute());
    let (store, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
    check!(eq; store.store_format()?.lifecycle_revision, Some(6));
    check!(eq; writer.qualification_version(), PublicIdentityQualificationVersion::RetainedAuthorityContextV2);
    let athlete = subject()?;
    check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?, vec![athlete.clone()]);
    let head = writer.head(&store)?;
    check!(eq; head.sequence, 0);
    let snapshot = store.snapshot();
    write_precommit_sequence(&root, snapshot.sequence())?;
    writer.establish_source_bound(
        &store,
        OPERATION,
        &athlete.id.cast(),
        snapshot.revision(),
        &head,
    )?;
    Err("child returned without interruption at the selected identity boundary".into())
}

fn write_precommit_sequence(root: &Path, sequence: u64) -> TestResult {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(PRECOMMIT_SEQUENCE))?;
    file.write_all(&sequence.to_be_bytes())?;
    file.sync_all()?;
    Ok(())
}

pub(super) fn read_precommit_sequence(root: &Path) -> TestResult<u64> {
    let path = root.join(PRECOMMIT_SEQUENCE);
    let metadata = std::fs::symlink_metadata(&path)?;
    check!(metadata.is_file());
    check!(eq; metadata.len(), 8);
    let mut bytes = [0_u8; 8];
    std::fs::File::open(path)?.read_exact(&mut bytes)?;
    Ok(u64::from_be_bytes(bytes))
}
