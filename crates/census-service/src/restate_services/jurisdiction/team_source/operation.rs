use super::{key, TeamsSourceRequest};
use crate::restate_services::JobError;
use census_reconcile::identity::WorkflowIdentity;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Teams,
    SchoolContacts,
}

pub(crate) fn contacts_key(identity: &str) -> String {
    format!("{identity}/contacts/sidearm_staff")
}

pub(super) fn phase(operation: &str, request: &TeamsSourceRequest) -> Result<Phase, JobError> {
    if operation == key(request) {
        return Ok(Phase::Teams);
    }
    let jurisdiction = &request.jurisdiction;
    let identity = WorkflowIdentity::jurisdiction(
        jurisdiction.jurisdiction,
        jurisdiction.season,
        jurisdiction.revision,
    );
    if request.source == census_crawl::sidearm_staff::SOURCE_ID
        && operation == contacts_key(identity.as_str())
    {
        return Ok(Phase::SchoolContacts);
    }
    Err(JobError::Terminal {
        message: "source request identity does not match its durable phase key".to_owned(),
    })
}
