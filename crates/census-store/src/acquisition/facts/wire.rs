use super::*;

impl FactDecode for SlotBinding {
    type Historical = SlotBinding<VersionOne>;
    fn from_historical(value: Self::Historical) -> Self {
        Self {
            version: CaptureProtocolVersion::V1,
            slot: value.slot,
            work_id: value.work_id,
        }
    }
}
impl FactDecode for Reservation {
    type Historical = Reservation<VersionOne>;
    fn from_historical(value: Self::Historical) -> Self {
        Self {
            version: CaptureProtocolVersion::V1,
            attempt: value.attempt,
        }
    }
}
impl FactDecode for Armed {
    type Historical = Armed<VersionOne>;
    fn from_historical(value: Self::Historical) -> Self {
        Self {
            version: CaptureProtocolVersion::V1,
            dispatch: value.dispatch,
            request: value.request,
        }
    }
}
impl FactDecode for Observed {
    type Historical = Observed<VersionOne>;
    fn from_historical(value: Self::Historical) -> Self {
        Self {
            version: CaptureProtocolVersion::V1,
            dispatch: value.dispatch,
            response: value.response,
        }
    }
}
impl FactDecode for Prepared {
    type Historical = Prepared<VersionOne>;
    fn from_historical(value: Self::Historical) -> Self {
        Self {
            version: CaptureProtocolVersion::V1,
            attempt: value.attempt,
            dispatch: value.dispatch,
            capture: value.capture,
            disposition: value.disposition,
        }
    }
}
impl FactDecode for WorkResult {
    type Historical = WorkResult<VersionOne>;
    fn from_historical(value: Self::Historical) -> Self {
        Self {
            version: CaptureProtocolVersion::V1,
            occurrence: value.occurrence,
        }
    }
}
