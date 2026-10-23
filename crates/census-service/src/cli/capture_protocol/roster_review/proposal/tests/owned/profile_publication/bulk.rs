use super::*;
use census_service::census::profiles::{
    self, CaptureOutcome, Cursor, ProfileSource, Selection, Target,
};

#[tokio::test]
async fn owned_profile_bulk_selector_preserves_original_rows_scope_and_end_cursor() -> TestResult {
    let fixture = fixture::Fixture::new().await?;
    let (store, _writer) = Store::open_current_qualification_v3(&fixture.path)?.into_parts();
    let original =
        OwnedProviderSchoolPacketV1::from_packet(packet(&store, &fixture.reference)?).read()?;
    let expected = original
        .roster_verdict()
        .roster()
        .ok_or("readable roster")?
        .athletes
        .iter()
        .filter(|member| member.athlete().grad_year == GradYear::CO2027)
        .map(|member| member.athlete().athlete_id.as_str())
        .collect::<Vec<_>>();
    check!(!expected.is_empty());
    let baseline = (store.snapshot().sequence(), store.receipt_count()?);
    let mut remaining = expected.iter();
    let mut cursor: Option<Cursor> = None;
    let steps = expected
        .len()
        .checked_add(2)
        .ok_or("selection step bound")?;
    let mut ended = false;
    for _ in 0..steps {
        match profiles::select(
            &store,
            UsJurisdiction::Michigan,
            GradYear::CO2027,
            cursor.as_ref(),
        )? {
            Selection::Profile(target) => {
                check!(eq; Some(target.athlete_id.as_str()), remaining.next().copied());
                check!(eq; target.packet.as_str(), fixture.reference.as_str());
                cursor = Some(target.next);
            }
            Selection::Advance { next, .. } => cursor = Some(next),
            Selection::End => {
                ended = true;
                break;
            }
        }
    }
    check!(ended);
    check!(remaining.next().is_none());
    check!(matches!(
        profiles::select(
            &store,
            UsJurisdiction::Michigan,
            GradYear::CO2027,
            cursor.as_ref()
        )?,
        Selection::End
    ));
    check!(matches!(
        profiles::select(&store, UsJurisdiction::Colorado, GradYear::CO2027, None)?,
        Selection::End
    ));
    let bad = Cursor {
        packet: AcquisitionDigest::parse(&fixture.reference)?,
        member: usize::MAX,
        finished: false,
    };
    check!(profiles::select(
        &store,
        UsJurisdiction::Michigan,
        GradYear::CO2027,
        Some(&bad)
    )
    .is_err());
    let wrong_site = Cursor { member: 0, ..bad };
    check!(profiles::select(
        &store,
        UsJurisdiction::Colorado,
        GradYear::CO2027,
        Some(&wrong_site)
    )
    .is_err());
    check!(eq; (store.snapshot().sequence(), store.receipt_count()?), baseline);
    Ok(())
}

#[tokio::test]
async fn owned_profile_bulk_capture_replays_original_source_after_owner_reopen() -> TestResult {
    let fixture = fixture::Fixture::new().await?;
    let (store, writer) = Store::open_current_qualification_v3(&fixture.path)?.into_parts();
    let target = first(&store)?;
    let packet = packet(&store, &fixture.reference)?;
    let owner = writer.public_resource_owner(&store, UsJurisdiction::Michigan)?;
    retain(&store, &owner, &packet, &target.athlete_id)?;
    let fetcher = offline(&store, &fixture)?;
    let source = ProfileSource::new(&store, &writer, &fetcher, UsJurisdiction::Michigan, false);
    check!(matches!(
        source.capture(&target).await?,
        CaptureOutcome::Captured
    ));
    let baseline = (store.snapshot().sequence(), store.receipt_count()?);
    check!(matches!(
        source.capture(&target).await?,
        CaptureOutcome::Captured
    ));
    check!(eq; (store.snapshot().sequence(), store.receipt_count()?), baseline);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    store.flush()?;
    drop((fetcher, owner, writer, store));
    let (store, writer) = Store::open_current_qualification_v3(&fixture.path)?.into_parts();
    let fetcher = offline(&store, &fixture)?;
    let source = ProfileSource::new(&store, &writer, &fetcher, UsJurisdiction::Michigan, false);
    check!(matches!(
        source.capture(&target).await?,
        CaptureOutcome::Captured
    ));
    check!(eq; (store.snapshot().sequence(), store.receipt_count()?), baseline);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    check!(eq; store.walk_table(Table::Performances)?.rows, 0);
    Ok(())
}

fn first(store: &Store) -> TestResult<Target> {
    match profiles::select(store, UsJurisdiction::Michigan, GradYear::CO2027, None)? {
        Selection::Profile(target) => Ok(target),
        _ => Err("first actual Class of 2027 source member".into()),
    }
}

fn offline(store: &Store, fixture: &fixture::Fixture) -> TestResult<Fetcher> {
    Ok(Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        Default::default(),
        Vec::new(),
    )?
    .with_origin_locks(
        fixture
            .path
            .parent()
            .ok_or("fixture parent")?
            .join("origin-locks"),
    )
    .with_offline(true))
}
