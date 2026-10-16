use super::{resolved_id, shared_public_id, VerifiedPublicIdentityView};
use crate::export::store_lineage;
use crate::report::{ReportError, ReportResult};
use census_domain::model::{
    AthleteIdentityProjection, IdentityApplication, IdentityProjectionBuilder,
};
use census_domain::public_identity_history::{
    EstablishmentOrdinal, PublicResolution, ResolvedSuccessor,
};
use census_store::{Store, StoreSnapshot, Table};
use fixture::{capture, merge, prepare_merge, projection, source_births, Fixture, TestResult};

pub(crate) mod fixture;
mod historical;
mod output;

#[test]
fn historical_v1_qualification_cannot_authorize_current_publication() -> TestResult {
    invariant(
        super::require_current_qualification(
            super::PublicIdentityQualificationVersion::SourceBoundV1,
        ),
        "public identity publication requires retained-authority qualification V2",
    )
}

#[test]
fn source_bound_groups_publish_only_store_issued_public_ids_and_retain_unknown_candidates(
) -> TestResult {
    let fixture = source_births()?;
    let (view, lineage) = capture(&fixture.store)?;
    let [first, second, unknown] = &fixture.athletes;
    let [first_birth, second_birth] = &fixture.births;
    let expected = PublicResolution::Canonical(ResolvedSuccessor {
        public_id: first_birth.public_id().clone(),
        representative_candidate: first.id.cast(),
        established: EstablishmentOrdinal {
            sequence: 1,
            position: 0,
        },
    });
    check!(eq; view.store_identity(), fixture.writer.store_identity());
    check!(eq; view.head(), second_birth.head());
    check!(eq; view.snapshot_sequence(), lineage.snapshot_sequence);
    check!(eq; view.public_id(first.id.as_str()), Some(first_birth.public_id()));
    check!(eq; view.public_id(second.id.as_str()), Some(second_birth.public_id()));
    check!(ne; first_birth.public_id().as_str(), first.id.as_str());
    check!(eq; view.candidate_resolution(first.id.as_str()), Some(&expected));
    check!(eq; view.resolve_public(first_birth.public_id())?, expected);
    let group = view.group(first.id.as_str()).ok_or("first public group")?;
    check!(eq; group.canonical_candidate().as_str(), first.id.as_str());
    check!(eq; group.members().count(), 1);
    check!(eq; view.public_id(unknown.id.as_str()), None);
    check!(eq; view.candidate_resolution(unknown.id.as_str()), Some(&PublicResolution::Unknown));
    check!(eq; view.public_id("absent-candidate"), None);
    check!(eq; view.candidate_resolution("absent-candidate"), None);
    check!(eq; view.correction_history(first_birth.public_id()), Some([].as_slice()));
    check!(eq; view.affected_record_keys(3), None);
    let foreign = source_births()?;
    let [foreign_birth, _] = &foreign.births;
    check!(eq; view.resolve_public(foreign_birth.public_id())?, PublicResolution::Unknown);
    check!(eq; view.correction_history(foreign_birth.public_id()), None);
    Ok(())
}

#[test]
fn real_reviewed_merge_maps_both_candidate_aliases_to_oldest_public_birth_across_reopen(
) -> TestResult {
    let fixture = source_births()?;
    let (original, _) = capture(&fixture.store)?;
    let merged = merge(&fixture)?;
    let (current, _) = capture(&fixture.store)?;
    let [first, second, unknown] = &fixture.athletes;
    let [first_birth, second_birth] = &fixture.births;
    let successor = ResolvedSuccessor {
        public_id: first_birth.public_id().clone(),
        representative_candidate: first.id.cast(),
        established: EstablishmentOrdinal {
            sequence: 1,
            position: 0,
        },
    };
    let redirect = PublicResolution::Redirect {
        successor,
        changed_at: 3,
    };
    check!(eq; merged.introduced_public_ids(), &[]);
    check!(eq; current.head(), merged.head());
    check!(eq; current.public_id(first.id.as_str()), Some(first_birth.public_id()));
    check!(eq; current.public_id(second.id.as_str()), Some(first_birth.public_id()));
    check!(eq; current.candidate_resolution(second.id.as_str()), Some(&redirect));
    check!(eq; current.resolve_public(second_birth.public_id())?, redirect);
    check!(eq; original.public_id(second.id.as_str()), Some(second_birth.public_id()));
    let canonical_candidate = first.id.as_str();
    let group = current
        .group(second.id.as_str())
        .ok_or("merged public group")?;
    check!(eq; group.canonical_candidate().as_str(), canonical_candidate);
    check!(eq; group.members().count(), 2);
    check!(eq; current.public_id(unknown.id.as_str()), None);
    let Fixture {
        directory,
        store,
        writer,
        athletes,
        births,
        ..
    } = fixture;
    drop(writer);
    drop(store);
    let (store, _writer) =
        Store::open_public_identity_qualification_v2(directory.path().join("identity"))?
            .into_parts();
    let (reopened, _) = capture(&store)?;
    let [first, second, _] = &athletes;
    let [first_birth, second_birth] = &births;
    check!(eq; reopened.head(), merged.head());
    check!(eq; reopened.public_id(first.id.as_str()), Some(first_birth.public_id()));
    check!(eq; reopened.public_id(second.id.as_str()), Some(first_birth.public_id()));
    check!(eq; reopened.resolve_public(second_birth.public_id())?, redirect);
    check!(eq; reopened.candidate_resolution(second.id.as_str()), current.candidate_resolution(second.id.as_str()));
    check!(eq; original.public_id(second.id.as_str()), Some(second_birth.public_id()));
    Ok(())
}

#[test]
fn automatic_aliases_do_not_replace_distinct_committed_public_identities() -> TestResult {
    let fixture = source_births()?;
    prepare_merge(&fixture)?;
    let snapshot = fixture.store.snapshot();
    let lineage = store_lineage(&fixture.store)?;
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("public ledger")?;
    let head = accepted.head().clone();
    let projection = accepted_review_projection(&snapshot)?;
    let view = VerifiedPublicIdentityView::capture(
        accepted,
        &lineage,
        &head,
        &snapshot.athletes()?,
        &projection,
    )?;
    let [first, second, _] = &fixture.athletes;
    let [first_birth, second_birth] = &fixture.births;
    check!(eq; projection.canonical_id(first.id.as_str()), projection.canonical_id(second.id.as_str()));
    check!(eq; view.public_id(first.id.as_str()), Some(first_birth.public_id()));
    check!(eq; view.public_id(second.id.as_str()), Some(second_birth.public_id()));
    check!(eq; view.group(first.id.as_str()).ok_or("group")?.members().count(), 1);
    check!(eq; view.group(second.id.as_str()).ok_or("group")?.members().count(), 1);
    check!(view.aliases().is_empty());
    check!(eq; view.candidate_resolution(first.id.as_str()), Some(&PublicResolution::Canonical(ResolvedSuccessor {
        public_id: first_birth.public_id().clone(), representative_candidate: first.id.cast(),
        established: EstablishmentOrdinal { sequence: 1, position: 0 },
    })));
    check!(eq; view.candidate_resolution(second.id.as_str()), Some(&PublicResolution::Canonical(ResolvedSuccessor {
        public_id: second_birth.public_id().clone(), representative_candidate: second.id.cast(),
        established: EstablishmentOrdinal { sequence: 2, position: 0 },
    })));
    Ok(())
}

#[test]
fn public_mapping_refuses_foreign_lineage_moved_snapshot_and_wrong_registry_head() -> TestResult {
    let fixture = source_births()?;
    let foreign = source_births()?;
    let snapshot = fixture.store.snapshot();
    let current = store_lineage(&fixture.store)?;
    let foreign_lineage = store_lineage(&foreign.store)?;
    let athletes = snapshot.athletes()?;
    let projection = projection(&snapshot)?;
    let [_, second_birth] = &fixture.births;
    for (lineage, head) in [
        (&foreign_lineage, second_birth.head()),
        (&fixture.initial_lineage, second_birth.head()),
        (&current, &fixture.initial_head),
        (
            &current,
            foreign.births.last().ok_or("foreign birth")?.head(),
        ),
    ] {
        let accepted = snapshot
            .accepted_public_identity_snapshot()?
            .ok_or("public ledger")?;
        invariant(
            VerifiedPublicIdentityView::capture(accepted, lineage, head, &athletes, &projection),
            "public identity view differs from the exact dataset lineage or registry head",
        )?;
    }
    Ok(())
}

#[test]
fn public_mapping_refuses_duplicate_rows_and_missing_canonical_group_members() -> TestResult {
    let fixture = source_births()?;
    let snapshot = fixture.store.snapshot();
    let lineage = store_lineage(&fixture.store)?;
    let [first, _, _] = &fixture.athletes;
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("public ledger")?;
    let head = accepted.head().clone();
    invariant(
        VerifiedPublicIdentityView::capture(
            accepted,
            &lineage,
            &head,
            &[first.clone(), first.clone()],
            &projection(&snapshot)?,
        ),
        "public identity candidate inventory repeats a subject",
    )?;
    drop(snapshot);
    let merged = merge(&fixture)?;
    let snapshot = fixture.store.snapshot();
    let lineage = store_lineage(&fixture.store)?;
    let projection = projection(&snapshot)?;
    let [_, alias, _] = &fixture.athletes;
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("public ledger")?;
    invariant(
        VerifiedPublicIdentityView::capture(
            accepted,
            &lineage,
            merged.head(),
            std::slice::from_ref(alias),
            &projection,
        ),
        "public identity candidate group has an absent canonical member",
    )?;
    Ok(())
}

#[test]
fn resolution_reduction_preserves_ambiguity_and_revocation_without_candidate_fallback() -> TestResult
{
    let fixture = source_births()?;
    let (view, _) = capture(&fixture.store)?;
    let [first, second, _] = &fixture.athletes;
    let PublicResolution::Canonical(first) = view
        .candidate_resolution(first.id.as_str())
        .ok_or("first resolution")?
        .clone()
    else {
        return Err("first birth is not canonical".into());
    };
    let PublicResolution::Canonical(second) = view
        .candidate_resolution(second.id.as_str())
        .ok_or("second resolution")?
        .clone()
    else {
        return Err("second birth is not canonical".into());
    };
    let canonical = PublicResolution::Canonical(first.clone());
    let other = PublicResolution::Canonical(second.clone());
    let redirected = PublicResolution::Redirect {
        successor: first.clone(),
        changed_at: 3,
    };
    let revoked = PublicResolution::RevokedRedirect {
        successor: first.clone(),
        changed_at: 4,
        revoked_merges: vec![3],
    };
    let ambiguous = PublicResolution::Ambiguous {
        successors: vec![first.clone(), second],
        unresolved_bindings: false,
        changed_at: 4,
        revoked_merges: vec![3],
    };
    for resolution in [&canonical, &redirected, &revoked] {
        check!(eq; resolved_id(resolution), Some(&first.public_id));
        check!(eq; shared_public_id([&canonical, resolution].into_iter()), Some(&first.public_id));
    }
    for resolution in [
        &ambiguous,
        &PublicResolution::Unresolved,
        &PublicResolution::Unknown,
    ] {
        check!(eq; resolved_id(resolution), None);
        check!(eq; shared_public_id([&canonical, resolution].into_iter()), None);
    }
    check!(eq; shared_public_id([&canonical, &other].into_iter()), None);
    check!(eq; shared_public_id(std::iter::empty()), None);
    Ok(())
}

fn accepted_review_projection(
    snapshot: &StoreSnapshot<'_>,
) -> TestResult<AthleteIdentityProjection> {
    let cases = snapshot.scan(Table::ReviewCases)?;
    let verdicts = snapshot.scan(Table::IdentityVerdicts)?;
    let mut builder =
        IdentityProjectionBuilder::new(snapshot.athlete_identity_index()?, &cases, &verdicts)?;
    let decisions = builder
        .reviewed_applications("2026-10-14")
        .map(|(_, application)| match application? {
            IdentityApplication::Accepted(accepted) => Ok(accepted.record().clone()),
            IdentityApplication::Retained(issue) => {
                Err(format!("review fixture retained: {issue:?}").into())
            }
        })
        .collect::<TestResult<Vec<_>>>()?;
    check!(eq; decisions.len(), 1);
    for decision in &decisions {
        check!(eq; builder.consider(decision)?, None);
    }
    Ok(builder.finish()?)
}

fn invariant<T>(result: ReportResult<T>, expected: &str) -> TestResult {
    match result {
        Err(ReportError::Invariant { detail }) => check!(eq; detail, expected),
        Err(error) => return Err(format!("unexpected public mapping refusal: {error:?}").into()),
        Ok(_) => return Err(format!("expected public mapping refusal: {expected}").into()),
    }
    Ok(())
}
