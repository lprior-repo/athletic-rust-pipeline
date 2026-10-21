use super::*;

pub(super) const URL: &str = "https://mi.milesplit.com/athletes/13749840-brennan-smith";
pub(super) const WHOLE: &str =
    "https://mi.milesplit.com/api/v1/meets/123/performances?fields=fixture";
pub(super) const START: &str = "2026-10-21T08:00:00Z";
pub(super) const RECEIVED: &str = "2026-10-21T08:00:01Z";
pub(super) const DONE: &str = "2026-10-21T08:00:02Z";

pub(super) fn identity() -> TestResult<StoreIdentity> {
    Ok(StoreIdentity::parse(&"A".repeat(64))?)
}

fn scope(admission: SourceAdmissionV1) -> TestResult<ResultsCaptureScopeV1> {
    Ok(ResultsCaptureScopeV1::new(
        "jurisdiction:MI:2026-27:1",
        UsJurisdiction::Michigan,
        CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("run")?,
        AcquisitionDigest::parse(&"a".repeat(64))?,
        SelectedHistoryV1::new(HistoryWindow::new(2023, 2026, "2026-10-21")?, 2026)?,
        admission,
        CaptureCacheIntent::ReuseEligible,
    )?)
}

pub(super) fn public() -> TestResult<AcquisitionWorkV1> {
    let scope = PublicResourceCaptureScopeV1::new(
        "jurisdiction:MI:2026-27:1",
        UsJurisdiction::Michigan,
        CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("run")?,
        AcquisitionDigest::parse(&"a".repeat(64))?,
        HistoryWindow::new(2023, 2026, "2026-10-21")?,
        AcquisitionDigest::parse(&"b".repeat(64))?,
        CaptureCacheIntent::ReuseEligible,
    )?;
    Ok(AcquisitionWorkV1::for_public_resource(
        identity()?,
        &scope,
        UsJurisdiction::Michigan,
        PublicResourceObligationV1::new(
            PublicSourceResourceV1::milesplit_athlete_profile("13749840")?,
            ProfileDate::parse("2026-10-21")?,
        ),
        RequestRepresentationV1::normalized_get(
            URL,
            vec![
                ("accept-encoding".into(), "identity".into()),
                ("user-agent".into(), "Census/1.0".into()),
            ],
            "Census/1.0",
        )?,
    )?)
}

pub(super) fn whole() -> TestResult<AcquisitionWorkV1> {
    Ok(AcquisitionWorkV1::for_owned_meet(
        identity()?,
        &scope(SourceAdmissionV1::milesplit_results(&"b".repeat(64))?)?,
        UsJurisdiction::Michigan,
        "123",
        RequestRepresentationV1::get(WHOLE, Vec::new(), "Census/1.0")?,
    )?)
}

pub(super) fn dispatch(work: &AcquisitionWorkV1, hop: u8) -> TestResult<AcquisitionDispatchId> {
    Ok(AcquisitionDispatchId::new(
        AcquisitionAttemptId::new(identity()?, work.id()?, AttemptOrdinal::new(1)?),
        HopOrdinal::new(hop)?,
    ))
}

pub(super) fn headers() -> TestResult<Vec<ObservedHttpHeaderV1>> {
    Ok(vec![
        ObservedHttpHeaderV1::new("accept-encoding", b"identity".to_vec())?,
        ObservedHttpHeaderV1::new("user-agent", b"Census/1.0".to_vec())?,
    ])
}

pub(super) fn request(
    target: &str,
    started: &str,
    headers: Vec<ObservedHttpHeaderV1>,
    conditional: ConditionalRequestV1,
) -> TestResult<DispatchRequestV1> {
    Ok(DispatchRequestV1::new_observed(
        target,
        conditional,
        ObservedHttpRequestV1::new(target, started, headers)?,
    )?)
}

pub(super) fn response(
    target: &str,
    received: &str,
    status: u16,
    headers: Vec<ObservedHttpHeaderV1>,
) -> TestResult<ResponseObservationV1> {
    let selected = headers
        .iter()
        .filter(|header| {
            matches!(
                header.name(),
                "content-type" | "etag" | "last-modified" | "location" | "retry-after"
            )
        })
        .map(|header| {
            Ok((
                header.name().to_owned(),
                std::str::from_utf8(header.value())?.to_owned(),
            ))
        })
        .collect::<TestResult<Vec<_>>>()?;
    Ok(ResponseObservationV1::new_observed(
        status,
        target,
        received,
        selected,
        ObservedHttpResponseV1::new(
            ObservedHttpProtocolV1::Http2,
            status,
            target,
            received,
            headers,
        )?,
    )?)
}

pub(super) fn observed_response() -> TestResult<ResponseObservationV1> {
    response(
        URL,
        RECEIVED,
        200,
        vec![
            ObservedHttpHeaderV1::new("content-type", b"text/html".to_vec())?,
            ObservedHttpHeaderV1::new("etag", b"source-etag".to_vec())?,
            ObservedHttpHeaderV1::new("x-source", vec![128])?,
        ],
    )
}

pub(super) fn normalized(
    dispatch: AcquisitionDispatchId,
    request: &DispatchRequestV1,
    response: &ResponseObservationV1,
) -> TestResult<NormalizedObservedHttpV1> {
    Ok(NormalizedObservedHttpV1::new(
        vec![ObservedHttpHopV1::new(
            dispatch,
            request.observed_http().ok_or("observed request")?.clone(),
            response.observed_http().ok_or("observed response")?.clone(),
        )?],
        DONE,
        AcquisitionDigest::parse(&"c".repeat(64))?,
        4,
    )?)
}

pub(super) fn prepared(observed: &NormalizedObservedHttpV1) -> TestResult<PreparedCaptureInputV1> {
    let response = observed
        .hops()
        .last()
        .ok_or("prepared response")?
        .response();
    Ok(PreparedCaptureInputV1::new(
        observed.body_digest().as_str(),
        observed.body_bytes(),
        serde_json::to_vec(&json!({
            "url": URL,
            "response_url": response.response_url(),
            "method": "GET",
            "representation": public()?.request().representation_headers(),
            "status": response.status(),
            "content_digest": observed.body_digest(),
            "bytes": observed.body_bytes(),
            "fetched_at": response.received_at(),
            "etag": selected(response, "etag")?,
            "last_modified": selected(response, "last-modified")?,
            "content_type": selected(response, "content-type")?,
            "redirects": [],
            "observed_http": observed,
        }))?,
    )?)
}

fn selected(response: &ObservedHttpResponseV1, name: &str) -> TestResult<Option<String>> {
    Ok(response
        .headers()
        .iter()
        .find(|header| header.name() == name)
        .map(|header| std::str::from_utf8(header.value()))
        .transpose()?
        .map(str::to_owned))
}

pub(super) fn facts(
    dispatch: AcquisitionDispatchId,
    request: DispatchRequestV1,
    response: ResponseObservationV1,
) -> AttemptFacts {
    AttemptFacts {
        reservation: Reservation {
            version: CaptureProtocolVersion::V2,
            attempt: dispatch.attempt().clone(),
        },
        dispatches: vec![(
            Armed {
                version: CaptureProtocolVersion::V2,
                dispatch: dispatch.clone(),
                request,
            },
            Some(Observed {
                version: CaptureProtocolVersion::V2,
                dispatch,
                response,
            }),
        )],
        prepared: None,
        terminal: None,
        receipt: ReceiptWitness::Absent,
    }
}

pub(super) struct Fixture {
    pub(super) work: AcquisitionWorkV1,
    pub(super) facts: AttemptFacts,
    pub(super) normalized: NormalizedObservedHttpV1,
    pub(super) prepared: PreparedCaptureInputV1,
}

pub(super) fn fixture() -> TestResult<Fixture> {
    let work = public()?;
    let dispatch = dispatch(&work, 0)?;
    let request = request(URL, START, headers()?, ConditionalRequestV1::Unconditional)?;
    let response = observed_response()?;
    let normalized = normalized(dispatch.clone(), &request, &response)?;
    let prepared = prepared(&normalized)?;
    Ok(Fixture {
        work,
        facts: facts(dispatch, request, response),
        normalized,
        prepared,
    })
}
