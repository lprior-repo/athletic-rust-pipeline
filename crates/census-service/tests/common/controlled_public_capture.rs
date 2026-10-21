use census_domain::model::*;
use census_store::{
    AcquisitionReadBudget, CaptureWriterV1, OwnedAcquisitionWork, PublicResourceCaptureOwnerV1,
    Store,
};
use sha2::{Digest, Sha256};

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct ControlledNormalizedCapture<'a> {
    pub body: &'a [u8],
    pub content_type: &'a str,
    pub started_at: &'a str,
    pub received_at: &'a str,
    pub completed_at: &'a str,
}

pub struct PreparedControlledNormalizedCapture<'a> {
    work: AcquisitionWorkV1,
    input: ControlledNormalizedCapture<'a>,
    observation: Observations,
}

pub fn prepare<'a>(
    work: &AcquisitionWorkV1,
    input: ControlledNormalizedCapture<'a>,
) -> TestResult<PreparedControlledNormalizedCapture<'a>> {
    let observation = Observations::new(work, &input)?;
    Ok(PreparedControlledNormalizedCapture {
        work: work.clone(),
        input,
        observation,
    })
}

pub fn retain_owned(
    store: &Store,
    owned: OwnedAcquisitionWork,
    prepared: PreparedControlledNormalizedCapture<'_>,
) -> TestResult<AcquisitionOccurrenceRefV1> {
    if owned.work() != &prepared.work {
        return Err("controlled normalized capture work differs".into());
    }
    finish(store, owned, prepared.input, prepared.observation)
}

pub fn retain_root(
    store: &Store,
    writer: &CaptureWriterV1,
    owner: &PublicResourceCaptureOwnerV1,
    work: AcquisitionWorkV1,
    input: ControlledNormalizedCapture<'_>,
) -> TestResult<AcquisitionOccurrenceRefV1> {
    let prepared = prepare(&work, input)?;
    let grant = writer.continuation_for_public_resource(store, owner, &work)?;
    let owned = store.register_acquisition_work(owner, work, grant)?;
    retain_owned(store, owned, prepared)
}

fn finish(
    store: &Store,
    owned: OwnedAcquisitionWork,
    input: ControlledNormalizedCapture<'_>,
    observation: Observations,
) -> TestResult<AcquisitionOccurrenceRefV1> {
    let work = owned.work().clone();
    let reserved = store.reserve_acquisition_attempt(owned)?;
    let armed = store.arm_acquisition_dispatch(reserved, observation.request.clone())?;
    let prepared_input = metadata(&work, armed.dispatch_id().clone(), &observation, &input)?;
    let response =
        store.record_acquisition_response(armed.into_observation(), observation.response)?;
    let prepared = store.prepare_acquisition_capture(
        response,
        prepared_input,
        CaptureDispositionV1::for_http_status(200)?,
    )?;
    publish(store, prepared.prepared(), input.body)?;
    let reference = store.finish_acquisition_capture(prepared)?;
    let retained =
        store.read_retained_acquisition_v1(&reference, &mut AcquisitionReadBudget::for_work())?;
    check!(eq; std::fs::read(retained.body_path())?, input.body);
    Ok(reference)
}

struct Observations {
    request: DispatchRequestV1,
    response: ResponseObservationV1,
}

impl Observations {
    fn new(work: &AcquisitionWorkV1, input: &ControlledNormalizedCapture<'_>) -> TestResult<Self> {
        let url = work.request().original_url();
        let headers = work
            .request()
            .representation_headers()
            .iter()
            .map(|(name, value)| ObservedHttpHeaderV1::new(name, value.as_bytes().to_vec()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            request: DispatchRequestV1::new_observed(
                url,
                ConditionalRequestV1::Unconditional,
                ObservedHttpRequestV1::new(url, input.started_at, headers)?,
            )?,
            response: ResponseObservationV1::new_observed(
                200,
                url,
                input.received_at,
                vec![("content-type".to_string(), input.content_type.to_string())],
                ObservedHttpResponseV1::new(
                    ObservedHttpProtocolV1::Http2,
                    200,
                    url,
                    input.received_at,
                    vec![ObservedHttpHeaderV1::new(
                        "content-type",
                        input.content_type.as_bytes().to_vec(),
                    )?],
                )?,
            )?,
        })
    }
}

fn metadata(
    work: &AcquisitionWorkV1,
    dispatch: AcquisitionDispatchId,
    observation: &Observations,
    input: &ControlledNormalizedCapture<'_>,
) -> TestResult<PreparedCaptureInputV1> {
    let digest = AcquisitionDigest::parse(&format!("{:x}", Sha256::digest(input.body)))?;
    let bytes = u64::try_from(input.body.len())?;
    let observed = NormalizedObservedHttpV1::new(
        vec![ObservedHttpHopV1::new(
            dispatch,
            observation
                .request
                .observed_http()
                .ok_or("controlled request observation")?
                .clone(),
            observation
                .response
                .observed_http()
                .ok_or("controlled response observation")?
                .clone(),
        )?],
        input.completed_at,
        digest.clone(),
        bytes,
    )?;
    let content_type = observation
        .response
        .headers()
        .iter()
        .find(|(name, _)| name == "content-type")
        .map(|(_, value)| value)
        .ok_or("controlled content type")?;
    let mut metadata = serde_json::json!({
        "url": work.request().original_url(),
        "response_url": observation.response.response_url(),
        "method": "GET",
        "representation": work.request().representation_headers(),
        "status": 200,
        "content_digest": digest,
        "bytes": bytes,
        "fetched_at": observation.response.observed_at(),
        "content_type": content_type,
        "observed_http": observed,
    });
    metadata.sort_all_objects();
    Ok(PreparedCaptureInputV1::new(
        digest.as_str(),
        bytes,
        serde_json::to_vec_pretty(&metadata)?,
    )?)
}

fn publish(store: &Store, input: &PreparedCaptureInputV1, body: &[u8]) -> TestResult<()> {
    let bodies = store.http_cache_dir().join("archive/bodies");
    let captures = store
        .http_cache_dir()
        .join("archive/captures")
        .join(input.content_digest().as_str());
    std::fs::create_dir_all(&bodies)?;
    std::fs::create_dir_all(&captures)?;
    std::fs::write(
        bodies.join(format!("{}.body", input.content_digest())),
        body,
    )?;
    std::fs::write(
        captures.join(format!("{}.meta.json", input.metadata_digest())),
        input.metadata_bytes(),
    )?;
    Ok(())
}
