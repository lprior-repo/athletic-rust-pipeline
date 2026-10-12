use super::*;

pub async fn collect_state_meet_fragment(
    fetcher: &Fetcher,
    store: &Store,
    request: &MeetWalkRequest<'_>,
    recording: Option<&Recording>,
) -> CrawlResult<MeetCensus> {
    let request = request.fragment();
    let site = Site::for_jurisdiction(request.jurisdiction);
    let sink = recording.map_or(RowSink::Store(store), |recording| RowSink::Record {
        store,
        recording,
    });
    let (census, _) = stream::iter(Season::ALL.into_iter().map(Ok))
        .try_fold(
            (MeetCensus::default(), false),
            |(census, acquired), season| {
                let request = &request;
                async move {
                    let phase = meets_phase(request.jurisdiction, season, request.year);
                    let progress = progress::SeasonProgress::load(store, &phase)?;
                    let acquire =
                        !acquired && progress.disposition == DiscoveryDisposition::Partial;
                    let walk = if acquire {
                        walk_season(
                            fetcher,
                            store,
                            SeasonReader {
                                site,
                                season,
                                request,
                            },
                            sink,
                        )
                        .await?
                    } else {
                        SeasonWalk {
                            rows: progress.rows_seen,
                            frontier: Some(MeetFrontier {
                                phase,
                                next_page: progress.next_page,
                                disposition: progress.disposition,
                                failure: None,
                            }),
                            ..SeasonWalk::default()
                        }
                    };
                    Ok::<_, census_crawl::CrawlError>((census.absorb(walk)?, acquired || acquire))
                }
            },
        )
        .await?;
    Ok(census.completed_index(site, &request))
}
