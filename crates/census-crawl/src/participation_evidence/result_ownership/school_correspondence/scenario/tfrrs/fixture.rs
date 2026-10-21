use super::*;

pub(super) struct Input {
    pub school: SchoolId,
    pub packet: ProviderSchoolPacketV1,
    pub context: String,
}

pub(super) fn install() -> TestResult<(tempfile::TempDir, Store, Input)> {
    let (directory, store, context) = install_controlled_tfrrs_result_attachments()?;
    let packet = provider()?;
    let recording = Recording::new();
    let school = record_provider_school_intake(&store, &recording, &packet)?;
    crate::recording::prepare_recorded(
        recording.drain(),
        "controlled-cross-provider-tfrrs-school-target",
    )?
    .apply(&store)?;
    setup::enroll(&store)?;
    Ok((
        directory,
        store,
        Input {
            school,
            packet,
            context,
        },
    ))
}

fn provider() -> TestResult<ProviderSchoolPacketV1> {
    let root = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/participation_evidence/provider_school/fixtures/"
    );
    let index = std::fs::read(format!("{root}mi-team-index.html"))?;
    let index_headers = std::fs::read(format!("{root}mi-team-index.headers"))?;
    let roster = std::fs::read(format!("{root}dcc-provider-roster.html"))?;
    let roster_headers = std::fs::read(format!("{root}dcc-provider-roster.headers"))?;
    Ok(ProviderSchoolPacketV1::new(
        setup::capture(
            "https://mi.milesplit.com/teams",
            &index,
            &index_headers,
            "2026-10-16T02:57:54Z",
            "2026-10-16T02:58:00Z",
        )?,
        setup::capture(
            "https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster",
            &roster,
            &roster_headers,
            "2026-10-16T00:49:38Z",
            "2026-10-16T00:49:43Z",
        )?,
    )?)
}

pub(super) fn proposal(
    store: &Store,
    input: &Input,
    index: usize,
    scope: ResultSchoolScopeV1,
) -> TestResult<ResultSchoolSourceProposalV1> {
    Ok(proposal_with_grant(
        store,
        input,
        index,
        scope,
        &mut AttachmentReplayGrantV1::new(),
    )?)
}

pub(super) fn proposal_with_grant(
    store: &Store,
    input: &Input,
    index: usize,
    scope: ResultSchoolScopeV1,
    grant: &mut AttachmentReplayGrantV1,
) -> crate::CrawlResult<ResultSchoolSourceProposalV1> {
    let snapshot = store.snapshot();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(grant)?;
    let reference = input.packet.reference()?;
    let provider = read_recorded_provider_school(&snapshot, &reference, grant)?
        .ok_or_else(|| missing("recorded provider target"))?;
    let discovered = discover_current_html_results(&current, grant)?;
    if discovered.group_count() != 1 || discovered.diagnostic_count() != 0 {
        return Err(missing("exact single capture group"));
    }
    discovered.with_group(0, grant, |linked, grant| {
        build_claim(&current, &linked, index, &provider, scope, grant)
    })
}

pub(super) fn verify_claim(
    proposal: &ResultSchoolSourceProposalV1,
    input: &Input,
    index: usize,
) -> TestResult {
    let claim = proposal.claim();
    let scope = scope_for(index)?;
    let expected = ["Columbus North", "Hamilton Southeastern"]
        .get(index)
        .copied()
        .ok_or("source team index")?;
    check!(eq; claim.statement.statement_text.as_str(), expected);
    check!(eq; claim.target.school, input.school);
    check!(eq; claim.target.day, scope.day);
    check!(eq; claim.target.sport, scope.sport);
    check!(eq; claim.target.category, scope.category);
    check!(eq; claim.target.year.get(), 2025);
    check!(eq; claim.statement.parser_contract.digest, digest(RESULT_SCHOOL_TFRRS_PARSER_CONTRACT_V1)?);
    let capture = common_fixture::capture(CheckedHtmlResultSourceV1::Tfrrs)?;
    let body = capture.body()?;
    let start = usize::try_from(claim.statement.statement_offset)?;
    let length = usize::try_from(claim.statement.statement_length)?;
    let end = start.checked_add(length).ok_or("statement end")?;
    check!(eq; body.get(start..end), Some(expected));
    check!(eq; claim.statement.statement_digest, digest(expected.as_bytes())?);
    Ok(())
}

pub(super) fn originals(store: &Store, input: &Input) -> TestResult<Vec<String>> {
    let snapshot = store.snapshot();
    let mut original = Vec::new();
    for (schema, key) in [
        (
            RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
            input.context.clone(),
        ),
        (
            RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
            input.packet.reference()?.as_str().to_owned(),
        ),
    ] {
        let raw = snapshot
            .journal_payload_raw_bounded(
                schema.phase(),
                &key,
                JournalReadBudget::new(4 * 1024 * 1024)?,
            )?
            .ok_or("retained original root")?;
        original.push(raw.get().to_owned());
    }
    original.push(serde_json::to_string(
        &snapshot.scan::<CanonicalAthlete>(Table::Athletes)?,
    )?);
    original.push(serde_json::to_string(
        &snapshot.scan::<CanonicalSchool>(Table::Schools)?,
    )?);
    Ok(original)
}
