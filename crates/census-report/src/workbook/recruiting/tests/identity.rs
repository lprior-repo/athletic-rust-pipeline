use super::*;
use census_domain::model::{
    AttestationQualification, IdentityApplication, IdentityAttestation, IdentityProjectionBuilder,
    IdentityStatus, ReviewCase, ReviewState, ReviewVerdictRecord, ATHLETE_IDENTITY_FAMILY,
};

mod published_cohort;

fn member(school: &SchoolId, name: &str, gender: Gender, url: &str) -> CanonicalAthlete {
    let mut athlete = CanonicalAthlete::new(
        school,
        name,
        GradYear::CO2027,
        gender,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9001001").with_url(url),
    );
    athlete.public_profile_urls.push(url.to_owned());
    athlete.evidence = evidence("wiaa_results", Some(url));
    athlete
}

pub(super) fn accept(store: &Store, members: &[CanonicalAthlete]) -> TestResult {
    check!(eq; apply_identity_pairs(store, &[members])?, 1);
    Ok(())
}

fn apply_identity_pairs(store: &Store, pairs: &[&[CanonicalAthlete]]) -> TestResult<u64> {
    let index = store.athlete_identity_index()?;
    let mut cases = Vec::new();
    let mut verdicts = Vec::new();
    for (ordinal, members) in pairs.iter().enumerate() {
        let ids = members
            .iter()
            .map(|athlete| athlete.id.cast())
            .collect::<Vec<_>>();
        let subject = "Source-backed identity fixture";
        let detail =
            format!("One published provider identifier retained with alternate names {ordinal}");
        let evidence = index.case_evidence(subject, &detail, &ids)?;
        let mut case = ReviewCase::pending_with_evidence(
            ATHLETE_IDENTITY_FAMILY,
            members
                .first()
                .ok_or("identity fixture has no members")?
                .id
                .as_str(),
            subject,
            &detail,
            evidence,
        );
        case.member_ids = ids;
        case.state = ReviewState::Resolved;
        let verdict = ReviewVerdictRecord {
            id: case.id.clone(),
            case_id: case.id.clone(),
            subject_id: case.subject_id.clone(),
            family: case.family.clone(),
            kind: "value_proposed".into(),
            field: "identity".into(),
            value: "same_person".into(),
            accepted: true,
            confidence: 100,
            rationale: detail,
            reviewer: "deterministic fixture".into(),
            observed_at: DAY.into(),
            member_ids: case.member_ids.clone(),
        };
        store.replace(Table::ReviewCases, &case)?;
        store.replace(Table::IdentityVerdicts, &verdict)?;
        cases.push(case);
        verdicts.push(verdict);
    }
    let builder = IdentityProjectionBuilder::new(index, &cases, &verdicts)?;
    let decisions = builder
        .reviewed_applications(DAY)
        .map(|(_, result)| -> TestResult<_> {
            match result? {
                IdentityApplication::Accepted(decision) => Ok(decision),
                IdentityApplication::Retained(issue) => {
                    Err(format!("fixture acceptance rejected: {issue:?}").into())
                }
            }
        })
        .collect::<TestResult<Vec<_>>>()?;
    store
        .apply_identity_decisions(&decisions)
        .map_err(Into::into)
}

fn add_result(
    store: &Store,
    athlete: &CanonicalAthlete,
    kind: EventKind,
    value: i32,
) -> TestResult {
    let meet = meet(
        store,
        UsJurisdiction::Wisconsin,
        "Union Invitational",
        "2026-05-01",
        CompetitionLevel::Invitational,
        Sport::OutdoorTrack,
    )?;
    let event = event(store, &meet, kind.clone())?;
    performance(
        store,
        &PerformanceRow {
            athlete: &athlete.id,
            school: &athlete.school,
            meet: &meet,
            event: &event,
            date: "2026-05-01",
        },
        Mark::TimeSeconds(ExactSeconds::from_parts(
            i64::from(value)
                .checked_mul(10_000_000)
                .ok_or("fixture time overflow")?,
            2,
        )?),
        "wiaa_results",
        "https://wiaa.test/union/results",
    )
}

#[test]
fn accepted_aliases_publish_one_person_with_all_events_profiles_and_supported_metadata(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school(&store, UsJurisdiction::Wisconsin, "Union High")?;
    let mut first = member(
        &school,
        "Synthetic Runner",
        Gender::Unknown,
        "https://wi.milesplit.com/athletes/9001001",
    );
    let mut second = member(
        &school,
        "S. Runner",
        Gender::Boys,
        "https://mn.milesplit.com/athletes/9001001",
    );
    first.public_profile_urls = vec!["https://profiles.test/first".into()];
    second.public_profile_urls = vec!["https://profiles.test/second".into()];
    first.sports = vec![Sport::OutdoorTrack];
    second.sports = vec![Sport::CrossCountry];
    first.observed_grades.push(ObservedGrade {
        grade: Grade::new(11).ok_or("invalid fixture grade")?,
        school_year: SchoolYear::new(2025).ok_or("invalid fixture season")?,
        source: SourceRef::id("wiaa_results"),
    });
    second.observed_grades.push(ObservedGrade {
        grade: Grade::new(12).ok_or("invalid fixture grade")?,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        source: SourceRef::id("wiaa_results"),
    });
    second.add_identity(
        SourceIdentity::new(SourceNamespace::athletic_net("athlete"), "union")
            .with_url("https://www.athletic.net/athlete/union"),
    );
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    add_result(&store, &first, EventKind::Track400m, 5000)?;
    add_result(&store, &second, EventKind::Track800m, 12000)?;
    coach(
        &store,
        &school,
        "Published Coach",
        Some(Sport::OutdoorTrack),
        CoachRole::HeadCoach,
        "coach@union.test",
    )?;
    accept(&store, &[first.clone(), second.clone()])?;
    let dataset = crate::export::ExportDataset::load(&store)?;
    let canonical = dataset
        .identities()
        .canonical_id(first.id.as_str())
        .to_owned();
    check!(eq; dataset.identities().canonical_id(second.id.as_str()),
    canonical);
    let derivation = crate::report::Derivation::of(&dataset, Scope::AllSources, Some(2027));
    let [athlete] = derivation.athletes() else {
        return Err("one accepted identity required".into());
    };
    check!(eq; athlete.id.as_str(), canonical);
    check!(eq; athlete.gender, Gender::Boys);
    for member in [&first, &second] {
        check!(athlete.known_names.contains(&member.canonical_name));
        check!(athlete
            .public_profile_urls
            .contains(&member.public_profile_urls[0]));
        check!(athlete.evidence.contains(&member.evidence[0]));
        check!(athlete.observed_grades.contains(&member.observed_grades[0]));
    }
    check!(eq; dataset.athletes.len(), 2);
    let coverage = crate::report::coverage_report(&dataset, Some(2027))?;
    let census = crate::report::build_census(&derivation, &store.out_dir());
    check!(eq; coverage.read.athletes, 1);
    check!(eq; coverage.read.performances, 2);
    check!(eq; coverage.published_totals().athletes, census.totals.athletes);
    let jurisdiction = coverage
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction == UsJurisdiction::Wisconsin.into())
        .ok_or("missing Wisconsin coverage")?;
    check!(eq; jurisdiction.with_performance, 1);
    check!(eq; jurisdiction.performances, 2);
    check!(eq; jurisdiction.athletes_core, 1);
    let projection = recruiting(&store, Scope::AllSources, Some(2027))?;
    let path = dir.path().join("accepted.xlsx");
    let mut book = Workbook::new();
    projection.write_athletes(&mut book, &path)?;
    projection.write_prs(&mut book, &path)?;
    book.save(&path)?;
    let mut book = open_workbook(&path)?;
    let range = sheet(&mut book, "Athletes")?;
    check!(eq; range.height(), 2);
    let row = row_of(&range, &canonical)?;
    for (header, expected) in [
        ("TF", "yes"),
        ("XC", "yes"),
        ("Event list", "400m; 800m"),
        ("Observed School Year", "2026-27"),
        ("Performance count", "2"),
        ("Meet count", "1"),
        ("Identity Status", "verified"),
        ("Head TF Coach", "Published Coach"),
        ("School", "Union High"),
        ("Sources Count", "2"),
    ] {
        check!(eq; text(&range, row, column_of(&range, header)?),
        expected,
        "{header}");
    }
    let summary = text(&range, row, column_of(&range, "Headline PR summary")?);
    check!(summary.contains("400m 50.00"));
    check!(summary.contains("800m 2:00.00"));
    let profiles = ["MileSplit URL", "Other profile URLs", "Athletic.net URL"]
        .into_iter()
        .map(|header| Ok(text(&range, row, column_of(&range, header)?)))
        .collect::<TestResult<Vec<_>>>()?
        .join("; ");
    check!(profiles.contains(&first.public_profile_urls[0]));
    check!(profiles.contains(&second.public_profile_urls[0]));
    check!(profiles.contains("https://wi.milesplit.com/athletes/9001001"));
    check!(profiles.contains("https://mn.milesplit.com/athletes/9001001"));
    check!(profiles.contains("https://www.athletic.net/athlete/union"));
    let prs = sheet(&mut book, "PRs")?;
    check!(eq; prs.height(), 3);
    check!(eq; text(&prs, row_of_event(&prs, &canonical, "Track400m")?, 9),
    "50.00");
    check!(eq; text(&prs, row_of_event(&prs, &canonical, "Track800m")?, 9),
    "2:00.00");
    verified_publication(&store, 1)?;
    Ok(())
}

#[test]
fn same_name_provider_owned_people_remain_separate_and_unresolved_status_is_explicit() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school(&store, UsJurisdiction::Wisconsin, "Homonym High")?;
    let mut first = member(
        &school,
        "Synthetic Runner",
        Gender::Boys,
        "https://wi.milesplit.com/athletes/9001001",
    );
    let mut second = CanonicalAthlete::new(
        &school,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "other-person"),
    );
    second.evidence = evidence("wiaa_results", Some("https://wiaa.test/other/results"));
    publish_fixture_cohort(&mut first, "wiaa_results", "provider-homonym-first", DAY);
    publish_fixture_cohort(&mut second, "wiaa_results", "provider-homonym-second", DAY);
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    store.replace(
        Table::ReviewCases,
        &ReviewCase::pending(
            ATHLETE_IDENTITY_FAMILY,
            second.id.as_str(),
            "Synthetic Runner",
            "Unresolved source ownership",
        ),
    )?;
    let dataset = crate::export::ExportDataset::load(&store)?;
    check!(dataset.canonical_aliases.is_empty());
    check!(eq; dataset.identities().status(first.id.as_str())?,
    IdentityStatus::Unverified);
    check!(eq; dataset.identities().status(second.id.as_str())?,
    IdentityStatus::Pending);
    let projection = recruiting(&store, Scope::AllSources, Some(2027))?;
    let path = dir.path().join("homonyms.xlsx");
    let mut book = Workbook::new();
    projection.write_athletes(&mut book, &path)?;
    book.save(&path)?;
    let mut book = open_workbook(&path)?;
    let range = sheet(&mut book, "Athletes")?;
    check!(eq; range.height(), 3);
    for (athlete, status) in [(&first, "unverified"), (&second, "pending")] {
        let row = row_of(&range, athlete.id.as_str())?;
        check!(eq; text(&range, row, column_of(&range, "Identity Status")?),
        status);
        check!(eq; text(&range, row, column_of(&range, "Review Status")?),
        "review");
    }
    Ok(())
}

fn attestation(
    subject: &SourceIdentity,
    publisher: &str,
    digest: char,
) -> TestResult<IdentityAttestation> {
    let claim = IdentityAttestation {
        subject: subject.clone(),
        source: SourceRef::new(publisher, Some(format!("https://{publisher}.test/capture"))),
        capture_sha256: digest.to_string().repeat(64),
        acquired_at: format!("{DAY}T12:00:00Z"),
        source_family: publisher.to_owned(),
        upstream_producer: format!("{publisher}-producer"),
        subject_locator: "published_subject".into(),
        qualification: AttestationQualification::IndependentPublished,
    };
    claim.validate()?;
    Ok(claim)
}

fn provider_member(
    school: &SchoolId,
    url: &str,
    source: SourceIdentity,
    links: &[SourceIdentity],
    claim: IdentityAttestation,
) -> CanonicalAthlete {
    let mut athlete = CanonicalAthlete::new(
        school,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        source,
    );
    athlete.public_profile_urls.push(url.to_owned());
    athlete.evidence = evidence("wiaa_results", Some(url));
    athlete.identity_attestations.push(claim);
    for link in links {
        athlete.add_identity(link.clone());
    }
    athlete
}

#[test]
fn conflicted_component_keeps_every_member_unmerged_in_both_orders() -> TestResult {
    for reverse in [false, true] {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let school = school(&store, UsJurisdiction::Wisconsin, "Union High")?;
        let key = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "7000009")
            .with_url("https://tfrrs.test/athletes/7000009");
        let mut first = provider_member(
            &school,
            "https://profiles.test/first",
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9001111")
                .with_url("https://wi.milesplit.com/athletes/9001111"),
            std::slice::from_ref(&key),
            attestation(&key, "first-fixture", 'a')?,
        );
        let mut owner = provider_member(
            &school,
            "https://profiles.test/owner",
            key.clone(),
            &[],
            attestation(&key, "owner-fixture", 'b')?,
        );
        let mut last = provider_member(
            &school,
            "https://profiles.test/last",
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9002222")
                .with_url("https://mn.milesplit.com/athletes/9002222"),
            std::slice::from_ref(&key),
            attestation(&key, "last-fixture", 'c')?,
        );
        for (member, purpose) in [
            (&mut first, "first"),
            (&mut owner, "owner"),
            (&mut last, "last"),
        ] {
            publish_fixture_cohort(
                member,
                "wiaa_results",
                &format!("conflicted-{purpose}"),
                DAY,
            );
        }
        store.append_many(
            Table::Athletes,
            &[first.clone(), owner.clone(), last.clone()],
        )?;
        for member in [&first, &owner, &last] {
            add_result(&store, member, EventKind::Track400m, 5000)?;
        }
        let mut pairs = [
            [first.clone(), owner.clone()],
            [owner.clone(), last.clone()],
        ];
        if reverse {
            pairs.reverse();
        }
        apply_identity_pairs(&store, &[pairs[0].as_slice(), pairs[1].as_slice()])?;
        let dataset = crate::export::ExportDataset::load(&store)?;
        check!(dataset.canonical_aliases.is_empty());
        check!(eq; dataset.athletes.len(), 3);
        for member in [&first, &owner, &last] {
            check!(eq; dataset.identities().canonical_id(member.id.as_str()),
            member.id.as_str());
            check!(eq; dataset.identities().status(member.id.as_str())?,
            IdentityStatus::RetainedConflict);
        }
        let derivation = crate::report::Derivation::of(&dataset, Scope::AllSources, Some(2027));
        check!(eq; derivation.athletes().len(), 3);
        let projection = recruiting(&store, Scope::AllSources, Some(2027))?;
        let path = dir.path().join("conflicted.xlsx");
        let mut book = Workbook::new();
        projection.write_athletes(&mut book, &path)?;
        projection.write_prs(&mut book, &path)?;
        book.save(&path)?;
        let mut book = open_workbook(&path)?;
        let range = sheet(&mut book, "Athletes")?;
        check!(eq; range.height(), 4);
        for member in [&first, &owner, &last] {
            let row = row_of(&range, member.id.as_str())?;
            check!(eq; text(&range, row, column_of(&range, "Identity Status")?),
            "retained_conflict");
            check!(eq; text(&range, row, column_of(&range, "Review Status")?), "review");
            check!(eq; text(&range, row, column_of(&range, "Performance count")?), "1");
            let profiles = ["MileSplit URL", "Other profile URLs", "Athletic.net URL"]
                .into_iter()
                .map(|header| Ok(text(&range, row, column_of(&range, header)?)))
                .collect::<TestResult<Vec<_>>>()?
                .join("; ");
            check!(profiles.contains(&member.public_profile_urls[0]));
            for other in [&first, &owner, &last] {
                check!(
                    other.id == member.id || !profiles.contains(&other.public_profile_urls[0]),
                    "another conflicted member profile must not merge into this row"
                );
            }
        }
        let prs = sheet(&mut book, "PRs")?;
        check!(eq; prs.height(), 4);
        for member in [&first, &owner, &last] {
            row_of_event(&prs, member.id.as_str(), "Track400m")?;
        }
    }
    Ok(())
}
