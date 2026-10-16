use super::*;
use census_store::read::JournalNamespace;

pub(super) fn emit(
    stage: &str,
    sources: &Sources,
    store: &Store,
    context: &context::Context,
) -> TestResult {
    let command = &context.command;
    let reviewed = &command.reviewed_context;
    let referenced_bytes =
        reviewed
            .objects
            .as_slice()
            .iter()
            .try_fold(0u64, |sum, reference| {
                sum.checked_add(reference.bytes)
                    .ok_or("reference byte sum overflow")
            })?;
    let inline_bytes = context
        .objects
        .0
        .iter()
        .filter_map(|material| match material {
            Material::Inline { bytes, .. } => Some(bytes.len()),
            Material::Retained(_) => None,
        })
        .try_fold(0usize, |sum, bytes| {
            sum.checked_add(bytes).ok_or("inline byte sum overflow")
        })?;
    let external = context
        .objects
        .0
        .iter()
        .filter(|material| matches!(material, Material::Retained(_)))
        .count();
    let mut oracle_grant = AttachmentReplayGrantV1::new();
    let control = command.approval_bytes_with_grant(4 * 1024 * 1024, &mut oracle_grant);
    let control = match control {
        Ok(bytes) => {
            json!({"outcome":"admitted", "approval_bytes":bytes.len(), "approval_sha256":hash(&bytes)})
        }
        Err(error) => json!({"outcome":"refused", "error":format!("{error:?}")}),
    };
    let summary = json!({"schema":"genuine-attachment-constructor-report/v1", "stage":stage,
        "claim":"Actual constructor outputs when this test runs; no authenticated human review or publication approval",
        "subjects":command.subjects.len(), "facts":reviewed.facts.len(), "pairs":reviewed.corroboration.len(),
        "scope_objects":reviewed.complete_scope_records.len(), "objects":reviewed.objects.len(),
        "target_subjects":reviewed.accepted_target_subjects.len(), "external_leaf_descriptors":external,
        "referenced_unique_bytes":referenced_bytes, "owned_inline_bytes":inline_bytes,
        "prior_objects":context.prior_objects.len(), "new_material_count":context.borrowed().len(),
        "complete_object_references":reviewed.objects.as_slice(), "command_sha256":hash(&serialized_bytes(command)?),
        "source_read_grant":{"encoded_bytes":sources.source_read_usage.encoded_bytes(),
            "steps":sources.source_read_usage.traversal_steps(), "scope":"construction read only; not workflow grant"},
        "independent_domain_command_control":{"result":control, "encoded_bytes":oracle_grant.usage().encoded_bytes(),
            "steps":oracle_grant.usage().traversal_steps(), "scope":"one actual command validation/approval encoding; not full workflow phase accounting"},
        "actual_workflow_grant_usage":"Opaque public API; not exposed or inferred from independent control",
        "physical_inventory":physical_inventory(store, sources)?});
    publish(&format!("{stage}-inventory"), &summary)?;
    publish(&format!("{stage}-command"), command)?;
    catalog(sources, &context.objects)
}

pub(super) fn catalog(sources: &Sources, objects: &Objects) -> TestResult {
    if let Some(root) = evidence_directory()? {
        for material in &objects.0 {
            let reference = material.reference();
            let path = root.join(format!("object-{}.bytes", reference.digest.as_str()));
            let bytes = match material {
                Material::Inline { bytes, .. } => bytes.as_slice(),
                Material::Retained(leaf) => {
                    let capture = sources
                        .captures
                        .iter()
                        .find(|capture| capture.json == leaf.object || capture.body == leaf.object)
                        .ok_or("retained emitter source")?;
                    match leaf.kind {
                        RetainedCaptureLeafKindV1::CaptureJson => &capture.original_json,
                        RetainedCaptureLeafKindV1::OriginalBody => &capture.original_body,
                    }
                }
            };
            check!(eq; super::reference(bytes)?, *reference);
            std::fs::write(path, bytes)?;
            if let Material::Retained(leaf) = material {
                std::fs::write(
                    root.join(format!("leaf-{}.json", reference.digest.as_str())),
                    serialized_bytes(leaf)?,
                )?;
            }
        }
    }
    Ok(())
}

pub(super) fn physical_inventory(store: &Store, sources: &Sources) -> TestResult<Value> {
    let snapshot = store.snapshot();
    let mut roots = Vec::new();
    for root in &sources.roots {
        let namespace = JournalNamespace::new(root.schema.phase(), &root.key)?;
        let inventory = snapshot.for_each_journal_root_bounded(
            namespace,
            preservation::journal_namespace_budget(),
            |_| Ok(()),
        )?;
        roots.push(json!({"root":root, "logical_roots":inventory.logical_roots(),
            "physical_rows":inventory.fingerprint().physical_rows(), "physical_encoded_bytes":inventory.fingerprint().encoded_bytes(),
            "chunk_rows":inventory.fingerprint().physical_rows().checked_sub(inventory.logical_roots()).ok_or("chunk inventory underflow")?}));
    }
    let attachments = snapshot.for_each_journal_root_bounded(
        JournalNamespace::new("evidence_subject", "attachment/v1/record/")?,
        preservation::journal_namespace_budget(),
        |_| Ok(()),
    )?;
    Ok(
        json!({"original_roots":roots, "attachment_roots":attachments.logical_roots(),
        "attachment_physical_rows":attachments.fingerprint().physical_rows(),
        "attachment_physical_bytes":attachments.fingerprint().encoded_bytes(), "receipt_count":store.receipt_count()?}),
    )
}

pub(super) fn phase(stage: &str, phase: &str, outcome: &str, detail: Value) -> TestResult {
    publish(
        &format!("{stage}-{phase}"),
        &json!({"schema":"genuine-attachment-workflow-observation/v1",
        "stage":stage, "phase":phase, "outcome":outcome, "detail":detail,
        "grant":"The same opaque inspection grant transfers into preparation and application; no reset or cap changes",
        "usage":"Internal per-phase counters are unavailable through the frozen public API"}),
    )
}

pub(super) fn publish(name: &str, value: &impl Serialize) -> TestResult {
    let bytes = serialized_bytes(value)?;
    eprintln!("GENUINE_ATTACHMENT {name} {}", std::str::from_utf8(&bytes)?);
    if let Some(directory) = evidence_directory()? {
        std::fs::write(directory.join(format!("{name}.json")), bytes)?;
    }
    Ok(())
}

fn evidence_directory() -> TestResult<Option<std::path::PathBuf>> {
    let Some(value) = std::env::var_os("CENSUS_GENUINE_ATTACHMENT_EVIDENCE_DIR") else {
        return Ok(None);
    };
    let directory = std::path::PathBuf::from(value);
    std::fs::create_dir_all(&directory)?;
    Ok(Some(directory))
}
