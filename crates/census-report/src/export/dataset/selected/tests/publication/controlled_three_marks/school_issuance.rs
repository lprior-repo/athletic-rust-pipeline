use super::source::{InstalledSource, RowLocator, TestResult};
use census_crawl::participation_evidence::provider_school::ProviderSchoolPacketRefV1;
use census_crawl::participation_evidence::recorded::discover_current_html_results;
use census_crawl::participation_evidence::recorded::read_recorded_provider_school;
use census_crawl::participation_evidence::result_ownership::school_correspondence::build_claim;
use census_crawl::participation_evidence::result_ownership::school_correspondence::ResultSchoolScopeV1;
use census_crawl::participation_evidence::result_ownership::school_correspondence::ResultSchoolSourceProposalV1;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::Gender;
use census_domain::model::ProfileDate;
use census_domain::model::SchoolYear;
use census_domain::result_school_correspondence::ResultSchoolActionV1;
use census_domain::result_school_correspondence::ResultSchoolPredecessorV1;
use census_domain::result_school_correspondence::ResultSchoolReviewActorV1;
use census_domain::result_school_correspondence::ResultSchoolReviewCommandV1;
use census_domain::result_school_correspondence::RESULT_SCHOOL_POLICY_V1;
use census_store::Application;
use census_store::Receipt;
use census_store::ResultSchoolCorrespondencePreparationV1;
use census_store::ResultSchoolEnrollmentReviewV1;
use census_store::RetainedCaptureRootSchemaV1;
use chrono::{DateTime, Utc};

pub(in super::super) fn enroll(
    store: &census_store::Store,
    operation: &str,
    rationale: &str,
) -> TestResult {
    if !matches!(store.store_format()?.lifecycle_revision, Some(7 | 8)) {
        return Err("controlled school enrollment requires checked predecessor7or8".into());
    }
    let inputs = store.inspect_result_school_enrollment_v1(AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_school_enrollment_v1(
        inputs,
        ResultSchoolEnrollmentReviewV1 {
            operation: text(operation)?,
            actor: ResultSchoolReviewActorV1::LocalOperator,
            reviewed_at: at()?,
            rationale: text(rationale)?,
        },
    )?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match store.apply_result_school_enrollment_v1(prepared, &approved, &digest)? {
        Application::Written(_) => {
            if store.store_format()?.lifecycle_revision != Some(9) {
                return Err("controlled school enrollment did not advance to9".into());
            }
            Ok(())
        }
        Application::Repeated(_) => Err("controlled enrollment unexpectedly replayed".into()),
    }
}

pub(super) fn issue(installed: &InstalledSource) -> TestResult<Vec<Receipt>> {
    let mut receipts = Vec::new();
    for (position, locator) in installed.row_locators.iter().enumerate() {
        let proposal = propose_for(installed, locator)?;
        if proposal.claim().target.school != installed.school {
            return Err("controlled school differs from installed school".into());
        }
        let operation = match position {
            0 => "controlled-school-accept-0",
            1 => "controlled-school-accept-1",
            _ => "controlled-school-accept-2",
        };
        receipts.push(accept_one(
            &installed.store,
            &proposal,
            operation,
            "Explicit protected original unique school-cell correspondence review exact performance day only",
        )?);
    }
    if receipts.len() != 3 {
        return Err("controlled issuance requires three receipts".into());
    }
    Ok(receipts)
}

fn propose_for(
    installed: &InstalledSource,
    locator: &RowLocator,
) -> TestResult<ResultSchoolSourceProposalV1> {
    let school_key = installed
        .source_records
        .iter()
        .find(|record| record.schema == RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1)
        .map(|record| record.key.as_str())
        .ok_or("provider school source absent")?;
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let provider = read_recorded_provider_school(
        &snapshot,
        &ProviderSchoolPacketRefV1::parse(school_key)?,
        &mut grant,
    )?
    .ok_or("recorded provider school absent")?;
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let discovered = discover_current_html_results(&current, &mut grant)?;
    if discovered.group_count() != 3 {
        return Err("controlled discovery requires three groups".into());
    }
    if discovered.diagnostic_count() != 0 {
        return Err("controlled discovery diagnostics present".into());
    }
    let mut index = 0usize;
    while index < discovered.group_count() {
        let found = discovered.with_group(index, &mut grant, |linked, grant| {
            if linked.recorded_root().key.as_str() != locator.document_key.as_str() {
                return Ok(None);
            }
            if linked.row_count() != 1 {
                return Err(census_store::StoreError::Refused {
                    detail: "controlled group row count".into(),
                }
                .into());
            }
            let row = linked.row(0).ok_or(census_store::StoreError::Refused {
                detail: "controlled group row absent".into(),
            })?;
            if row.checked_row().locator() != locator.locator {
                return Err(census_store::StoreError::Refused {
                    detail: "controlled original row locator differs".into(),
                }
                .into());
            }
            let scope = scope_for(linked.checked_document(), row.checked_row())?;
            let proposal = build_claim(&current, &linked, 0, &provider, scope, grant)?;
            Ok(Some(proposal))
        })?;
        if let Some(proposal) = found {
            return Ok(proposal);
        }
        index += 1;
    }
    Err("controlled group key unmatched".into())
}

pub(in super::super) fn scope_for(
    document: &census_crawl::participation_evidence::result_ownership::CheckedRetainedResultDocumentV1<'_>,
    row: &census_crawl::participation_evidence::result_ownership::CheckedRetainedResultRowV1<'_>,
) -> census_crawl::CrawlResult<ResultSchoolScopeV1> {
    let day = ProfileDate::parse(document.published_start()).map_err(|_| {
        census_store::StoreError::Refused {
            detail: "controlled performance day".into(),
        }
    })?;
    if document
        .published_end()
        .is_some_and(|end| end != document.published_start())
    {
        return Err(census_store::StoreError::Refused {
            detail: "controlled published end differs".into(),
        }
        .into());
    }
    let sport = document
        .parsed_page()
        .sport
        .ok_or(census_store::StoreError::Refused {
            detail: "controlled sport absent".into(),
        })?;
    let category = row.event().gender;
    if category == Gender::Unknown {
        return Err(census_store::StoreError::Refused {
            detail: "controlled category unknown".into(),
        }
        .into());
    }
    let year = SchoolYear::from_date(day.as_str()).ok_or(census_store::StoreError::Refused {
        detail: "controlled school year".into(),
    })?;
    if year != document.parsed_page().school_year {
        return Err(census_store::StoreError::Refused {
            detail: "controlled school year differs".into(),
        }
        .into());
    }
    Ok(ResultSchoolScopeV1 {
        day,
        sport,
        category,
        year,
    })
}

pub(in super::super) fn accept_one(
    store: &census_store::Store,
    proposal: &ResultSchoolSourceProposalV1,
    operation: &str,
    rationale: &str,
) -> TestResult<Receipt> {
    let inspection =
        store.inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new())?;
    let command = ResultSchoolReviewCommandV1 {
        version: 1,
        policy: text(RESULT_SCHOOL_POLICY_V1)?,
        store: inspection.store_identity().clone(),
        operation: text(operation)?,
        input_token: inspection.input_token().clone(),
        evidence_generation: inspection.evidence_generation(),
        derived_generation: inspection.derived_generation(),
        accepted_head: inspection.public_head().clone(),
        expected_inventory: inspection.inventory().clone(),
        source: proposal.claim().clone(),
        action: ResultSchoolActionV1::Accept {
            prior: ResultSchoolPredecessorV1::First,
        },
        actor: ResultSchoolReviewActorV1::LocalOperator,
        reviewed_at: at()?,
        rationale: text(rationale)?,
    };
    let context = proposal.borrowed_context(&mut AttachmentReplayGrantV1::new())?;
    let prepared = store
        .prepare_result_school_correspondence_v1(
            command,
            ResultSchoolCorrespondencePreparationV1::New {
                inspection,
                context: &context,
            },
        )
        .map_err(|e| format!("prepare controlled school review: {e}"))?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match store
        .apply_result_school_correspondence_v1(prepared, &approved, &digest)
        .map_err(|e| format!("commit controlled school review: {e}"))?
    {
        Application::Written(receipt) => Ok(receipt),
        Application::Repeated(_) => Err("controlled school review unexpectedly replayed".into()),
    }
}

fn text(
    value: &str,
) -> TestResult<census_domain::result_school_correspondence::ResultSchoolTextV1> {
    Ok(census_domain::result_school_correspondence::ResultSchoolTextV1::parse(value)?)
}

fn at() -> TestResult<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339("2026-10-17T00:00:00Z")?.with_timezone(&Utc))
}
