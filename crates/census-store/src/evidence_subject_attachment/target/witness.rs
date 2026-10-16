use super::*;

pub(in super::super) struct WitnessInputs<'a> {
    pub(in super::super) evidence_generation: u64,
    pub(in super::super) derived_generation: u64,
    pub(in super::super) capture: &'a AttachmentObjectReferenceV1,
    pub(in super::super) input_revisions: &'a AttachmentObjectReferenceV1,
}

#[derive(Serialize)]
struct Witness<'a> {
    schema: &'static str,
    store: &'a StoreIdentity,
    qualification: &'static str,
    public_head: &'a RegistryHead,
    evidence_generation: u64,
    derived_generation: u64,
    group: &'a AttachmentObjectReferenceV1,
    capture_binding: &'a AttachmentObjectReferenceV1,
    input_revisions: &'a AttachmentObjectReferenceV1,
}

pub(in super::super) fn encode_witness(
    target: &CurrentTarget<'_>,
    inputs: WitnessInputs<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<EncodedObject> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = checked(target, inputs, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn checked(
    target: &CurrentTarget<'_>,
    inputs: WitnessInputs<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<EncodedObject> {
    let accepted = target.accepted;
    grant.charge_steps(1).map_err(Error::Domain)?;
    if accepted.qualification_version()
        != PublicIdentityQualificationVersion::RetainedAuthorityGraphV3
        || inputs.capture.bytes == 0
        || inputs.input_revisions.bytes == 0
    {
        return Err(Error::Corrupt("target witness qualification or input references").into());
    }
    let wire = Witness {
        schema: "retained-evidence-subject-attachment-target-witness/v1",
        store: accepted.store_identity(),
        qualification: "retained_authority_graph_v3",
        public_head: accepted.head(),
        evidence_generation: inputs.evidence_generation,
        derived_generation: inputs.derived_generation,
        group: target.group.reference(),
        capture_binding: inputs.capture,
        input_revisions: inputs.input_revisions,
    };
    encoding::object(&wire, grant)
}
