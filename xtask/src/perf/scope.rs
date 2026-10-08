use super::compare::CAPTURE_EXPORT_WORKLOAD;
use anyhow::{bail, Result};

const CRITERION_SCOPE: &str = "criterion-per-iteration/v1;memory=whole-process-single-criterion-test/gnu-time-rss-kib+dhat-heap-json-v2-tbk-tb/v1";
const CAPTURE_SCOPE: &str = "capture-publication-independent-readback/v1;memory=whole-process-single-criterion-test/gnu-time-rss-kib+dhat-heap-json-v2-tbk-tb/v1";

pub(super) fn expected(id: &str) -> &'static str {
    if id == CAPTURE_EXPORT_WORKLOAD {
        CAPTURE_SCOPE
    } else {
        CRITERION_SCOPE
    }
}

pub(super) fn validate(id: &str, scope: Option<&str>, source: &str) -> Result<()> {
    let required = expected(id);
    if scope != Some(required) {
        bail!("{source} {id}: incompatible timing scope or memory backend; required {required}; recorded {scope:?}");
    }
    Ok(())
}
