use super::*;
use census_domain::model::{Gender, ProvisionalInclusionV1, Sport};

pub(in super::super) enum CapturedCase {
    Michigan,
    IllinoisBoys,
    IllinoisGirls,
    IllinoisVarsity,
}

impl CapturedCase {
    fn source(&self) -> (&str, &str, &str) {
        match self {
            Self::Michigan => (
                "https://or.milesplit.com/meets/696747/results/1189674/raw",
                "e4fc22921eca6c4db1609a4a825755b9cea7c31c54983ac127372bd9255aa6b1",
                "150e47846964c02aa2aabc1d8f4d5bcd17dacfff3c62dd414932e5cc7fc534b9",
            ),
            Self::IllinoisBoys => (
                "https://ia.milesplit.com/meets/571515/results/962542/raw",
                "5ee9c62e52458fdbc45e8ee5635009073b1552bfc7c0ccd275ac404c927a6d59",
                "121ea75901ecebeb5b249aaf6389514c59b59477871585b5c25c1e6b1dbfc101",
            ),
            Self::IllinoisGirls => (
                "https://ia.milesplit.com/meets/571515/results/962541/raw",
                "5ee9c62e52458fdbc45e8ee5635009073b1552bfc7c0ccd275ac404c927a6d59",
                "ed7b090f1c9ec811932f6d0c11563ebfb59a2ec49a09b0964f4dc642e5af16de",
            ),
            Self::IllinoisVarsity => (
                "https://ia.milesplit.com/meets/571515/results/962544/raw",
                "5ee9c62e52458fdbc45e8ee5635009073b1552bfc7c0ccd275ac404c927a6d59",
                "ee8dbefb41cfbb2316d94a9a2094ffe8c0b842af89b08fd1b3524a75cf3c56c4",
            ),
        }
    }
    fn state(&self) -> UsJurisdiction {
        match self {
            Self::Michigan => UsJurisdiction::Michigan,
            Self::IllinoisBoys | Self::IllinoisGirls | Self::IllinoisVarsity => {
                UsJurisdiction::Illinois
            }
        }
    }
    fn teams(&self) -> &[&str] {
        match self {
            Self::Michigan => &["12205", "22961"],
            Self::IllinoisBoys | Self::IllinoisGirls | Self::IllinoisVarsity => &["51713"],
        }
    }
    fn first_result(&self) -> &str {
        match self {
            Self::Michigan => "milesplit_result:190653353",
            Self::IllinoisBoys => "milesplit_result:150464273",
            Self::IllinoisGirls => "milesplit_result:150464434",
            Self::IllinoisVarsity => "milesplit_result:150463870",
        }
    }
}

impl Fixture {
    pub(in super::super) async fn captured(case: CapturedCase) -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("store");
        Store::create_public_identity_qualification_v3(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        for team in case.teams() {
            let mut school = CanonicalSchool::new(case.state(), *team, *team, None).0;
            school
                .source_identities
                .push(SourceIdentity::new(SourceNamespace::MilesplitSchool, *team));
            store.append(Table::Schools, &school)?;
        }
        collect(&case, &store, directory.path()).await?;
        let performance = store
            .scan::<CanonicalPerformance>(Table::Performances)?
            .into_iter()
            .find(|row| row.source_key == case.first_result())
            .ok_or("captured performance")?;
        let mut input = super::super::profile::profile(participation_policy_v4()?)?
            .input()
            .clone();
        input.jurisdictions = vec![case.state()];
        input.sports = vec![Sport::OutdoorTrack, Sport::CrossCountry];
        input.competition_categories = vec![Gender::Boys, Gender::Girls];
        input.include_provisional = ProvisionalInclusionV1::Include;
        qualify_profile(&mut input)?;
        let profile = QueryProfileV1::new(input)?;
        store.register_profile(&profile)?;
        for athlete in store.snapshot().athletes()? {
            writer.establish_source_bound(
                &store,
                &format!("captured-reported-{}", athlete.id),
                &athlete.id.cast(),
                store.snapshot().revision(),
                &writer.head(&store)?,
            )?;
        }
        Ok(Self {
            _directory: directory,
            store,
            performance,
            profile,
        })
    }
}

fn qualify_profile(input: &mut census_domain::model::QueryProfileInputV1) -> TestResult {
    use census_domain::profile_execution::{
        selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
    };
    input.source_plan = reference(Role::SourcePlan)?;
    input.completion_contract = reference(Role::Completion)?;
    input.policies = census_domain::model::ProfilePoliciesV1 {
        cohort: reference(Role::Cohort)?,
        event: reference(Role::Event)?,
        identity: reference(Role::Identity)?,
        affiliation: reference(Role::Affiliation)?,
        contact: reference(Role::Contact)?,
        participation: reference(Role::Participation)?,
        publication: reference(Role::Publication)?,
    };
    Ok(())
}

async fn collect(case: &CapturedCase, store: &Store, directory: &std::path::Path) -> TestResult {
    let fetcher = Fetcher::new(
        directory.join("http"),
        None,
        std::time::Duration::ZERO,
        HashMap::new(),
        vec!["milesplit.com".into()],
    )?
    .with_offline(true);
    let (url, owned, raw) = case.source();
    let reference = ResultSetRef::parse(url).ok_or("captured reference")?;
    let request = census_crawl::milesplit::owned_meet_request(
        &fetcher,
        &reference,
        &FetchOptions::default(),
    )?;
    cache::seed(&fetcher, request.original_url(), &bytes(owned)?)?;
    cache::seed(&fetcher, &reference.url, &bytes(raw)?)?;
    let context = AdapterContext {
        capture: CaptureContext::LegacyMetadata,
        fetcher: &fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        observed_on: "2026-10-22".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 22).ok_or("as of")?,
        recording: None,
    };
    let options = ResultSetOptions {
        urls: vec![ResultSetRequest {
            url: url.into(),
            jurisdiction: case.state(),
        }],
    };
    let acquisition: Pin<Box<dyn Future<Output = CrawlResult<AdapterReport>> + '_>> =
        Box::pin(collect_result_sets(&context, &options));
    acquisition.await?;
    Ok(())
}

fn bytes(digest: &str) -> TestResult<Vec<u8>> {
    use sha2::{Digest, Sha256};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/milesplit/reported-event-context-20261022")
        .join(format!("{digest}.body"));
    let bytes = std::fs::read(path)?;
    check!(eq; format!("{:x}", Sha256::digest(&bytes)), digest);
    Ok(bytes)
}
