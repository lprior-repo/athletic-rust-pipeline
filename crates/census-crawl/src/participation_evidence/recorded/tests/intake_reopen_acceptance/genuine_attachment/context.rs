use super::*;

mod objects;

use objects::{identity_scope, lineage, paired_facts, prior_objects, scoped_facts};

#[derive(Clone, Copy)]
pub(super) enum Stage {
    Article,
    Results,
}

pub(super) struct History {
    pub(super) records: Vec<ReviewedReferenceRecordV1>,
    pub(super) receipts: Vec<Receipt>,
    pub(super) inventory: AttachmentInventoryV1,
}

impl History {
    pub(super) fn read(store: &Store) -> TestResult<Self> {
        let snapshot = store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let retained = snapshot.read_evidence_subject_attachment_references_v1(&mut grant)?;
        Ok(Self {
            records: retained.records().to_vec(),
            receipts: retained.receipts().to_vec(),
            inventory: retained.inventory().clone(),
        })
    }

    pub(super) fn require_active_article(&self, store: &Store) -> TestResult {
        let bindings = self
            .records
            .iter()
            .map(|record| AttachmentReceiptBindingV1 {
                operation: record.command.operation.clone(),
                record_digest: record.record_digest.clone(),
            })
            .collect::<Vec<_>>();
        let projection = replay_reviewed_references(
            &store.snapshot().store_identity()?,
            &self.records,
            &bindings,
            &self.inventory,
        )?;
        let active = projection.active_records().collect::<Vec<_>>();
        let [article] = active.as_slice() else {
            return Err("B requires exact one active A record".into());
        };
        check!(eq; article.command.operation.as_str(), ARTICLE_OPERATION);
        check!(eq; article.command.subjects.len(), 1);
        check!(eq; article.command.subjects.as_slice().first().ok_or("A subject")?.role,
            AttachmentSubjectRoleV1::AthleteStatus);
        check!(eq; self.receipts.len(), self.records.len());
        for receipt in &self.receipts {
            check!(eq; store.receipt(&receipt.operation)?, Some(receipt.clone()));
        }
        Ok(())
    }
}

pub(super) struct Context {
    pub(super) objects: Objects,
    pub(super) command: RetainedEvidenceSubjectAttachmentCommandV1,
    pub(super) prior_objects: Vec<AttachmentObjectReferenceV1>,
}

impl Context {
    pub(super) fn build(
        stage: Stage,
        sources: &Sources,
        store: &Store,
        inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
        history: &History,
    ) -> TestResult<Self> {
        check!(eq; inspection.inventory(), &history.inventory);
        match stage {
            Stage::Article => check!(history.records.is_empty()),
            Stage::Results => history.require_active_article(store)?,
        }
        let mut objects = sources.objects.clone();
        let article_context = objects.json(&diagnostics::article(sources)?)?;
        let provider_context = objects.json(&diagnostics::provider(sources)?)?;
        let school_context = objects.json(&diagnostics::school(sources)?)?;
        let unavailable = objects.json(&diagnostics::unavailable_roster(sources)?)?;
        let raw_context = objects.json(&diagnostics::raw_results(sources)?)?;
        let identities = objects.json(&identity_scope(store, inspection)?)?;
        let subjects = subjects::build(stage, sources, &mut objects, article_context.clone())?;
        if matches!(stage, Stage::Results) {
            let prior = history.records.first().ok_or("actual active A")?;
            check!(eq; prior.command.target, *inspection.target());
            check!(eq; prior.command.subjects.as_slice(), std::slice::from_ref(&subjects.article));
            check!(prior
                .command
                .reviewed_context
                .facts
                .as_slice()
                .iter()
                .any(|fact| fact.fact == subjects.article_fact
                    && fact.subject == subjects.article.subject
                    && fact.disposition == AttachmentFactDispositionV1::SupportsAttachment));
        }
        let mhsaa = lineage(&mut objects, sources, "mhsaa", &["article"])?;
        let milesplit = lineage(
            &mut objects,
            sources,
            "milesplit",
            &["directory", "provider-roster", "profile", "chsl", "may15"],
        )?;
        let dcc = lineage(
            &mut objects,
            sources,
            "dccshamrocks",
            &["schedule", "dcc-roster"],
        )?;
        let mut facts = scoped_facts(
            &mut objects,
            sources,
            &subjects,
            inspection.accepted_target_witness(),
            &mhsaa,
            &milesplit,
            &dcc,
            [
                &article_context,
                &provider_context,
                &school_context,
                &unavailable,
                &raw_context,
                &identities,
            ],
        )?;
        let (mut attached, mut targets, pairs) =
            paired_facts(stage, &subjects, &mhsaa, &milesplit, &mut facts)?;
        attached.sort_by(|left, right| left.subject.cmp(&right.subject));
        targets.sort();
        targets.dedup();
        facts.sort_by(|left, right| left.fact.cmp(&right.fact));
        let prior_objects = prior_objects(history);
        let source_objects = objects.references();
        let captures = sources
            .captures
            .iter()
            .map(|capture| {
                json!({"key":capture.key,
            "capture":capture.json, "body":capture.body})
            })
            .collect::<Vec<_>>();
        let scope = objects.json(&json!({"schema":"genuine-attachment-complete-scope/v1",
            "authorship":"test-reviewed-context; not a source-issued identity or independent-producer attestation",
            "inventory_sha256":INVENTORY_DIGEST, "source_inventory_sha256":SOURCES_DIGEST,
            "stage":match stage { Stage::Article => "A", Stage::Results => "B" },
            "all_retained_roots":sources.roots, "all_original_capture_and_body_objects":captures,
            "all_source_objects":source_objects, "article_context":article_context, "provider_context":provider_context,
            "school_context":school_context, "unavailable_dcc_context":unavailable,
            "complete_result_context":raw_context, "accepted_target_context":identities,
            "all_facts_and_dispositions":facts, "all_proposed_subjects":attached, "all_pairs":pairs,
            "prior_attachment_inventory":history.inventory, "prior_records":history.records,
            "prior_receipts":history.receipts, "prior_objects":prior_objects,
            "inspection_objects":inspection.issued_objects().collect::<Vec<_>>(),
            "dependency_rule":"Finite earlier records only; no self-reference. Active A resolves original article context but is never a publisher.",
            "separate_gates":"Official school host, dated affiliation, exact event/season, result projection, report descriptors and frozen publication remain unapproved."}))?;
        let mut scopes = history
            .records
            .iter()
            .flat_map(|record| {
                record
                    .command
                    .reviewed_context
                    .complete_scope_records
                    .as_slice()
                    .iter()
                    .cloned()
            })
            .collect::<Vec<_>>();
        scopes.push(scope);
        scopes.sort();
        scopes.dedup();
        let mut references = objects.references();
        references.extend(prior_objects.iter().cloned());
        references.extend(inspection.issued_objects().cloned());
        references.sort();
        references.dedup();
        if matches!(stage, Stage::Results) {
            let mut directly_used = Vec::new();
            for subject in &attached {
                directly_used.extend([
                    subject.capture.clone(),
                    subject.body.clone(),
                    subject.interpretation.clone(),
                    subject.locator.clone(),
                    subject.subject.clone(),
                    subject.context.clone(),
                ]);
            }
            for fact in &facts {
                directly_used.extend([
                    fact.fact.clone(),
                    fact.subject.clone(),
                    fact.publisher_lineage.clone(),
                ]);
            }
            directly_used.extend(scopes.iter().cloned());
            directly_used.extend(targets.iter().cloned());
            directly_used.extend(
                inspection
                    .issued_objects()
                    .map(|reference| reference.digest.clone()),
            );
            references.retain(|reference| directly_used.contains(&reference.digest));
            for material in &objects.0 {
                check!(
                    references.contains(material.reference())
                        || prior_objects.contains(material.reference())
                );
            }
        }
        let combined = [
            attached.len(),
            references.len(),
            facts.len(),
            pairs.len(),
            scopes.len(),
            targets.len(),
        ]
        .into_iter()
        .try_fold(0usize, |sum, count| {
            sum.checked_add(count).ok_or("combined count overflow")
        })?;
        report::publish(
            match stage {
                Stage::Article => "A-before-bounded-conversion",
                Stage::Results => "B-before-bounded-conversion",
            },
            &json!({"schema":"genuine-attachment-construction-counts/v1", "subjects":attached.len(),
                "objects":references.len(), "facts":facts.len(), "pairs":pairs.len(), "scopes":scopes.len(),
                "target_subjects":targets.len(), "combined_references":combined,
                "complete_current_references":references, "complete_prior_references":prior_objects,
                "source_catalog_references":objects.references(), "full_source_bytes_retained":true,
                "claim":"Actual constructor inventory when executed; bounded conversion and workflow admission remain pending"}),
        )?;
        report::catalog(sources, &objects)?;
        let reviewed = ReviewedAttachmentContextV1 {
            objects: list(references)?,
            facts: list(facts)?,
            corroboration: list(pairs)?,
            complete_scope_records: list(scopes)?,
            accepted_target_subjects: list(targets)?,
            accepted_target_witness: inspection.accepted_target_witness().clone(),
        };
        let command = RetainedEvidenceSubjectAttachmentCommandV1 {
            version: 1, policy: text(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?, store: inspection.store_identity().clone(),
            operation: text(match stage { Stage::Article => ARTICLE_OPERATION, Stage::Results => RESULTS_OPERATION })?,
            expected_public_head: inspection.public_head().clone(), expected_evidence_generation: inspection.evidence_generation(),
            expected_derived_generation: inspection.derived_generation(), expected_attachment_inventory: inspection.inventory().clone(),
            target: inspection.target().clone(), subjects: list(attached)?, action: inspection.action().clone(), reviewed_context: reviewed,
            actor: AttachmentReviewActorV1::LocalOperator, reviewed_at: instant("2026-10-16T03:00:00Z")?,
            rationale: text("Explicit-review mechanism/capacity test over genuine originals; controlled approval bytes are not authenticated human approval of this identity attachment")?,
        };
        Ok(Self {
            objects,
            command,
            prior_objects,
        })
    }

    pub(super) fn borrowed(&self) -> Vec<AttachmentContextObjectV1<'_>> {
        self.objects
            .0
            .iter()
            .filter(|object| {
                self.command
                    .reviewed_context
                    .objects
                    .as_slice()
                    .contains(object.reference())
                    && !self.prior_objects.contains(object.reference())
            })
            .map(Material::borrowed)
            .collect()
    }
}
