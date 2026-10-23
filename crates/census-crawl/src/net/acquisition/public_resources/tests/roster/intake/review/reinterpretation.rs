use super::*;
use crate::net::assess_directory_roster_child;
use crate::net::public_http::{parse_url, TransportScope};
use crate::participation_evidence::provider_school::{
    OWNED_PROVIDER_SCHOOL_SEMANTIC_CONTRACT, PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2,
};
use census_domain::model::AcquisitionDigest;
use census_domain::roster_affiliation_authority::SourceRowLocator as DomainLocator;
use census_store::{
    AcquisitionReadBudget, PublicDirectoryEvidenceV1, PublicDirectoryRosterAssessmentV1,
    PublicDirectorySemanticContractV1,
};

struct PriorAssessment<'a, T> {
    current: &'a T,
    kernel: AcquisitionDigest,
    row_shift: u32,
}

impl<T: PublicDirectoryRosterAssessmentV1> PublicDirectoryRosterAssessmentV1
    for PriorAssessment<'_, T>
{
    fn evidence(&self) -> &PublicDirectoryEvidenceV1 {
        self.current.evidence()
    }
    fn team_id(&self) -> &str {
        self.current.team_id()
    }
    fn team_url(&self) -> &str {
        self.current.team_url()
    }
    fn roster_url(&self) -> &str {
        self.current.roster_url()
    }
    fn parsed_teams(&self) -> usize {
        self.current.parsed_teams()
    }
    fn row(&self) -> DomainLocator {
        let row = self.current.row();
        DomainLocator::new(
            row.ordinal().saturating_add(self.row_shift),
            row.byte_offset(),
            row.byte_length(),
        )
    }
    fn name(&self) -> DomainLocator {
        self.current.name()
    }
    fn city(&self) -> DomainLocator {
        self.current.city()
    }
    fn anchor(&self) -> DomainLocator {
        self.current.anchor()
    }
    fn row_digest(&self) -> &AcquisitionDigest {
        self.current.row_digest()
    }
    fn kernel_digest(&self) -> &AcquisitionDigest {
        &self.kernel
    }
    fn semantic_contract(&self) -> PublicDirectorySemanticContractV1 {
        self.current.semantic_contract()
    }
}

async fn prior_pair(
    fixture: &Fixture,
    listener: TcpListener,
    row_shift: u32,
) -> TestResult<(CapturedPublicDirectory, CapturedFetch, AcquisitionDigest)> {
    let acquire = async {
        let directory = fixture.acquire().await?;
        let original = directory
            .directory()
            .retained_observed()?
            .ok_or("directory")?;
        let evidence = fixture.owner.directory_evidence(
            &fixture.store,
            original.original(),
            &mut AcquisitionReadBudget::for_work(),
        )?;
        let current = assess_directory_roster_child(&evidence, directory.directory(), TEAM)?;
        let prior = PriorAssessment {
            current: &current,
            kernel: AcquisitionDigest::parse(&"1".repeat(64))?,
            row_shift,
        };
        check!(ne; current.kernel_digest(), prior.kernel_digest());
        let prepared = fixture.store.prepare_public_roster_child(&prior)?;
        let origin = parse_url(prepared.work().request().original_url())?.origin();
        let owned = fixture.store.register_public_roster_child(prepared)?;
        let roster = fixture
            .fetcher
            .get_acquisition_with_scope(
                &fixture.store,
                owned,
                &FetchOptions::default(),
                TransportScope::PublicHttp(&origin),
            )
            .await?;
        let kernel = current.kernel_digest().clone();
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>((directory, roster, kernel))
    };
    let (requests, pair) = tokio::try_join!(
        serve(listener, [Reply::directory(INDEX), roster_reply()]),
        acquire
    )?;
    check!(eq; requests.len(), 2);
    Ok(pair)
}

fn current_interpretation(held: &Held, current: &AcquisitionDigest) -> TestResult {
    let owned = OwnedProviderSchoolPacketV1::decode(&held.bytes)?;
    let checked = owned.read_semantic_contract(OWNED_PROVIDER_SCHOOL_SEMANTIC_CONTRACT)?;
    verify_school(&checked)?;
    check!(eq; checked.semantic_contract(), OWNED_PROVIDER_SCHOOL_SEMANTIC_CONTRACT);
    check!(eq; checked.reference().as_str(), held.packet.reference().as_str());
    check!(owned
        .read_semantic_contract(PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2)
        .is_err());
    check!(owned.read_semantic_contract("unknown").is_err());
    for evidence in &checked.school().evidence {
        let note: serde_json::Value = serde_json::from_str(
            evidence
                .note
                .as_deref()
                .ok_or("owned interpretation provenance")?,
        )?;
        check!(eq; note["directory_interpretation"]["retained_child_kernel"], "1".repeat(64));
        check!(eq; note["directory_interpretation"]["current_kernel"], current.as_str());
    }
    check!(eq; owned.encode()?, held.bytes);
    check!(eq; held.packet.child_evidence().kernel_digest().as_str(), "1".repeat(64));
    Ok(())
}

#[tokio::test]
async fn controlled_prior_kernel_packet_gets_current_review_and_store_free_frozen_replay(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = review_fixture(&listener)?;
    let (directory, roster, current) = prior_pair(&fixture, listener, 0).await?;
    let held = mint(&fixture, &directory, &roster)?;
    let before = Witness::capture(&fixture.store)?;
    current_interpretation(&held, &current)?;
    check!(eq; Witness::capture(&fixture.store)?, before);
    enroll(&fixture.store)?;
    let (receipt, digest) = issue(&fixture.store, &held)?;
    let original = freeze(&fixture.store, &held)?;
    current_interpretation(&held, &current)?;
    let fixture = fixture.reopen()?;
    let repeated = fixture
        .store
        .replay_roster_source_review_v1(OPERATION, &digest)?;
    check!(eq; repeated.receipt(), &receipt);
    let cold = freeze(&fixture.store, &held)?;
    check!(eq; cold.bytes(), original.bytes());
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    check!(eq; fixture.store.receipt(&held.receipt.operation)?, Some(held.receipt.clone()));
    let Fixture {
        root,
        store,
        writer,
        owner,
        fetcher,
    } = fixture;
    store.flush()?;
    drop((owner, writer, store, fetcher));
    root.close()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let replayed = replay_frozen_roster_reviewed_claims_v1(
        original.bytes(),
        original.reference(),
        &mut grant,
    )?;
    let claims = replayed.claims(&mut grant)?;
    reassess(
        claims.first().ok_or("frozen current owned claim")?,
        &held,
        &mut grant,
    )?;
    current_interpretation(&held, &current)
}

#[tokio::test]
async fn authenticated_prior_child_with_false_locator_refuses_current_interpretation_without_writes(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = review_fixture(&listener)?;
    let (directory, roster, _) = prior_pair(&fixture, listener, 1).await?;
    let prepared = prepare(&fixture, &directory, &roster)?;
    let packet = OwnedProviderSchoolPacketV1::from_packet(prepared.packet().clone());
    let bytes = packet.encode()?;
    let before = Witness::capture(&fixture.store)?;
    let refused = packet.read().err().ok_or("false child locator accepted")?;
    check!(refused
        .to_string()
        .contains("directory child locators differ"));
    check!(eq; packet.encode()?, bytes);
    check!(eq; Witness::capture(&fixture.store)?, before);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    Ok(())
}
