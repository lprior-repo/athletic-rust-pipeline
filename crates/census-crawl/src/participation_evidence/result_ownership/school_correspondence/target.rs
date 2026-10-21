use super::{invalid, objects::Objects, ResultSchoolScopeV1};
use crate::participation_evidence::provider_school::CheckedProviderSchoolPacketV1;
use crate::participation_evidence::recorded::{
    read_original_provider_school, RecordedProviderSchoolV1,
};
use crate::participation_evidence::result_ownership::{
    CheckedRetainedResultDocumentV1, CheckedRetainedResultRowV1,
};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{CanonicalSchool, Gender, SchoolYear, SourceSubject};
use census_domain::result_school_correspondence::{
    ResultSchoolTargetClaimV1, RESULT_SCHOOL_YEAR_CONTRACT_V1,
};
use census_store::{RetainedCaptureRootV1, StoreSnapshot, Table};

pub(super) fn scope_source(
    document: &super::super::CheckedHtmlResultDocumentV1<'_>,
    row: &super::super::CheckedHtmlResultRowV1<'_>,
    scope: &ResultSchoolScopeV1,
) -> CrawlResult<()> {
    use super::super::{CheckedHtmlResultDocumentV1 as Document, CheckedHtmlResultRowV1 as Row};
    match (document, row) {
        (Document::MileSplit(document), Row::MileSplit(row)) => self::scope(document, row, scope),
        (Document::Tfrrs(document), Row::Tfrrs(row)) => {
            if row.published_date() != scope.day.as_str()
                || document.sport() != scope.sport
                || row.gender() == Gender::Unknown
                || row.gender() != scope.category
                || SchoolYear::from_date(row.published_date()) != Some(scope.year)
            {
                return Err(invalid(
                    "TFRRS school date sport category year differs from original row",
                ));
            }
            Ok(())
        }
        _ => Err(invalid("school scope and original source differ")),
    }
}

pub(super) fn scope(
    document: &CheckedRetainedResultDocumentV1<'_>,
    row: &CheckedRetainedResultRowV1<'_>,
    scope: &ResultSchoolScopeV1,
) -> CrawlResult<()> {
    let day = crate::context::published_performance_date(document.published_start())
        .ok_or_else(|| invalid("school exact performance date unresolved"))?;
    if document
        .published_end()
        .is_some_and(|end| crate::context::published_performance_date(end) != Some(day))
        || day.format("%Y-%m-%d").to_string() != scope.day.as_str()
        || document.parsed_page().sport != Some(scope.sport)
        || row.event().gender == Gender::Unknown
        || row.event().gender != scope.category
        || SchoolYear::from_date(scope.day.as_str()) != Some(scope.year)
    {
        return Err(invalid(
            "school date sport category year differs from original result",
        ));
    }
    Ok(())
}

pub(super) fn build(
    snapshot: &StoreSnapshot<'_>,
    provider: &RecordedProviderSchoolV1,
    scope: ResultSchoolScopeV1,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ResultSchoolTargetClaimV1> {
    let original = snapshot
        .read_retained_capture_root_v1(provider.root().schema, &provider.root().key, grant)?
        .ok_or_else(|| invalid("school original provider packet absent"))?;
    if original.root() != provider.root() {
        return Err(invalid(
            "school provider packet differs from actual original",
        ));
    }
    let actual = read_original_provider_school(&original, grant)?;
    if actual.reference().as_str() != original.root().key {
        return Err(invalid("school provider packet original key differs"));
    }
    grant.charge_steps(1).map_err(super::domain)?;
    let installed: CanonicalSchool = snapshot
        .get_merged(Table::Schools, actual.school().id.as_str())?
        .ok_or_else(|| invalid("school provider target not installed"))?;
    checked(original.root(), &actual, &installed, scope, objects, grant)
}

pub(super) fn checked(
    root: &RetainedCaptureRootV1,
    provider: &CheckedProviderSchoolPacketV1,
    installed: &CanonicalSchool,
    scope: ResultSchoolScopeV1,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ResultSchoolTargetClaimV1> {
    if installed != provider.school() {
        return Err(invalid(
            "school installed projection differs from checked provider",
        ));
    }
    let observation = provider.observation();
    let source_school = SourceSubject::new(
        observation.namespace.clone(),
        observation.source_school_id.clone(),
    )?;
    if !installed.source_identities.iter().any(|identity| {
        identity.namespace == *source_school.namespace()
            && identity.id == source_school.provider_id()
    }) {
        return Err(invalid("school installed native provider identity absent"));
    }
    Ok(ResultSchoolTargetClaimV1 {
        school: installed.id.clone(),
        source_school,
        provider_school_root: objects.json(root, grant)?,
        physical_school_inventory: objects.json(installed, grant)?,
        day: scope.day,
        sport: scope.sport,
        category: scope.category,
        year: scope.year,
        year_contract: objects.bytes(RESULT_SCHOOL_YEAR_CONTRACT_V1, grant)?,
    })
}
