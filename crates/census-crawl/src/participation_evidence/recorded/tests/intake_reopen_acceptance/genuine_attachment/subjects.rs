use super::*;
use crate::participation_evidence::DocumentSubjectV1;

pub(super) struct Subjects {
    pub(super) article: RetainedDocumentSubjectReferenceV1,
    pub(super) article_fact: AttachmentDigestV1,
    pub(super) article_senior_hurdles: Option<AttachmentDigestV1>,
    pub(super) target_subjects: Vec<AttachmentDigestV1>,
    pub(super) target_facts: Vec<AttachmentDigestV1>,
    pub(super) results: Vec<RetainedDocumentSubjectReferenceV1>,
    pub(super) result_facts: Vec<AttachmentDigestV1>,
}

pub(super) fn build(
    stage: context::Stage,
    sources: &Sources,
    objects: &mut Objects,
    article_context: AttachmentDigestV1,
) -> TestResult<Subjects> {
    let checked = sources.participation.checked();
    let capture = sources.get("article")?;
    let proof = checked.status_proof();
    let interpretation = objects.bytes(
        format!(
            "mhsaa_subject_dcc_event_day_v1\n{}\n{}",
            proof.capture(),
            proof.locator()
        )
        .into_bytes(),
    )?;
    check!(eq; interpretation.as_str(), proof.interpretation());
    let article = subject(
        objects,
        checked.subject(),
        capture,
        interpretation,
        article_context,
        AttachmentSubjectRoleV1::AthleteStatus,
    )?;
    let article_fact = digest_span(
        objects,
        STATUS,
        113940,
        588,
        "d668ec9ee0510d228cb19e0bba2e08fe934eccfc80548280f3ec56329e1d0428",
    )?;
    let article_senior_hurdles = match stage {
        context::Stage::Article => None,
        context::Stage::Results => Some(objects.bytes(span(STATUS, 113861, 667)?.to_vec())?),
    };
    for (offset, length, digest) in [
        (
            113861,
            66,
            "5b6a7eb82af2852669b5b7df9b37804f90d89bcb065825d74e2dc84125c308f4",
        ),
        (
            106790,
            18605,
            "9f884851e60e2b85ab51eff97b7d1c48a3cb3224ad6e977d18fe03e80f5cf834",
        ),
        (
            107891,
            67,
            "56aaadc1f2d91abc85c3697b4aa83a48110698b0c3ce409d8aacde93f9b8f005",
        ),
    ] {
        check!(eq; hash(span(STATUS, offset, length)?), digest);
    }
    let (target_subjects, target_facts) = linked(sources, objects)?;
    let (results, result_facts) = match stage {
        context::Stage::Article => (Vec::new(), Vec::new()),
        context::Stage::Results => results(sources, objects)?,
    };
    Ok(Subjects {
        article,
        article_fact,
        article_senior_hurdles,
        target_subjects,
        target_facts,
        results,
        result_facts,
    })
}

fn subject(
    objects: &mut Objects,
    checked: &DocumentSubjectV1,
    capture: &Capture,
    interpretation: AttachmentDigestV1,
    context: AttachmentDigestV1,
    role: AttachmentSubjectRoleV1,
) -> TestResult<RetainedDocumentSubjectReferenceV1> {
    let preimage = format!(
        "document_subject_v1\n{}\n{}\n{}\n{}\n{}",
        checked.document_namespace(),
        checked.native_document_id(),
        checked.document_url(),
        checked.body_sha256(),
        checked.locator()
    );
    let subject = objects.bytes(preimage.into_bytes())?;
    check!(eq; subject.as_str(), checked.reference());
    check!(eq; capture.body.digest.as_str(), checked.body_sha256());
    Ok(RetainedDocumentSubjectReferenceV1 {
        namespace: text(checked.document_namespace())?,
        native_id: Some(text(checked.native_document_id())?),
        capture: capture.json.digest.clone(),
        body: capture.body.digest.clone(),
        interpretation,
        locator: objects.bytes(checked.locator().as_bytes().to_vec())?,
        subject,
        context,
        role,
    })
}

fn linked(
    sources: &Sources,
    objects: &mut Objects,
) -> TestResult<(Vec<AttachmentDigestV1>, Vec<AttachmentDigestV1>)> {
    let checked = sources.target.checked();
    let spans = [
        (
            65480usize,
            "13.26",
            "93fa4b8db2c428b36aacd581f89720d3dc49533e400064803178cb4f05f1453d",
        ),
        (
            69033,
            "36.75",
            "2a6e216d180730bc6724860d98a71312803f4a8473ec2ec75f444161257de2a4",
        ),
    ];
    check!(eq; checked.categories().rows().len(), spans.len());
    let mut subjects = Vec::new();
    let mut facts = Vec::new();
    for (row, (offset, mark, digest)) in checked.categories().rows().iter().zip(spans) {
        check!(eq; row.mark(), mark);
        check!(eq; row.meet_id(), "760382");
        check!(eq; row.result_document_id(), "1300204");
        let end = offset
            .checked_add(1319usize)
            .ok_or("CHSL locator overflow")?;
        check!(eq; row.proof().locator(), format!("html_bytes:{offset}:{end}"));
        let fact = digest_span(objects, RESULTS, offset, 1319, digest)?;
        let interpretation = objects.bytes(
            format!(
                "milesplit_profile_linked_competition_target_v1\n{}\n{}",
                row.proof().capture(),
                row.proof().locator()
            )
            .into_bytes(),
        )?;
        check!(eq; interpretation.as_str(), row.proof().interpretation());
        let locator = objects.bytes(row.proof().locator().as_bytes().to_vec())?;
        let subject = objects.json(&json!({"schema":"genuine-attachment-provider-linked-subject/v1",
            "authorship":"test DTO of checked published person link; no provider result-row identifier",
            "person":checked.profile().person(), "school":checked.profile().school(),
            "name":checked.profile().name(), "packet_reference":checked.reference().as_str(),
            "capture":sources.get("chsl")?.json, "body":sources.get("chsl")?.body,
            "row_fact":fact, "locator":locator, "interpretation":interpretation,
            "proof":diagnostics::provider_proof(row.proof()),
            "event_code":row.event_code(), "result_document_id":row.result_document_id(),
            "result_document_url":row.result_document_url(), "meet_id":row.meet_id()}))?;
        subjects.push(subject);
        facts.push(fact);
    }
    Ok((subjects, facts))
}

fn results(
    sources: &Sources,
    objects: &mut Objects,
) -> TestResult<(
    Vec<RetainedDocumentSubjectReferenceV1>,
    Vec<AttachmentDigestV1>,
)> {
    let document = sources.result.read(&locators())?;
    let page = document.parsed_page();
    let context = objects.json(&(
        "retained-milesplit-html-result-row/v1",
        document.capture(),
        &page.meet,
        &page.skipped,
        &page.grade_issues,
        document.published_start(),
        document.published_end(),
    ))?;
    check!(eq; context.as_str(), document.context_reference());
    let digests = [
        "bc7c7f83bcfc715c9c3343e09cf45f82bee7651ac86838f3427e1097de010b3d",
        "6bf39ca146977ced76ba63e226c1d41786e1914947f492c148fe3ca8708cd582",
        "8595ae83c23900e31b747b8bc0b3c6adb7c842139e0344f15e361a71e718d736",
    ];
    let mut subjects = Vec::new();
    let mut facts = Vec::new();
    check!(eq; document.selected_count(), 3);
    for (index, expected) in digests.into_iter().enumerate() {
        let row = document.row(index).ok_or("complete requested row")?;
        let original = row.original_row()?.as_bytes();
        check!(eq; hash(original), expected);
        check!(eq; row.original_row_digest(), expected);
        let fact = objects.bytes(original.to_vec())?;
        let interpretation = objects.json(&(
            "retained-milesplit-html-result-row/v1",
            document.capture().requested_url(),
            document.capture().body_sha256(),
            row.locator(),
        ))?;
        check!(eq; interpretation.as_str(), row.reference());
        let reference = subject(
            objects,
            &row.subject()?,
            sources.get("may15")?,
            interpretation,
            context.clone(),
            AttachmentSubjectRoleV1::ResultRow,
        )?;
        subjects.push(reference);
        facts.push(fact);
    }
    Ok((subjects, facts))
}
