use super::{
    harness, publication, readback, Boundary, Case, Intent, Path, Root, Store, TestResult,
};
use crate::report::ReportError;

#[derive(Clone, Copy)]
enum Damage {
    MissingProof,
    ChangedProof,
    PartialArchive,
}

pub(super) fn refuse_each_damage(
    case: &Case,
    store: &Store,
    intent: &Intent,
    target: &Path,
    boundary: Boundary,
) -> TestResult {
    let generation = target.parent().ok_or("desired generation")?;
    let proof = std::fs::read_dir(generation.join("frozen-input.json.proofs"))?
        .next()
        .ok_or("selected proof object")??
        .path();
    for damage in [
        Damage::MissingProof,
        Damage::ChangedProof,
        Damage::PartialArchive,
    ] {
        let path = match damage {
            Damage::MissingProof | Damage::ChangedProof => proof.clone(),
            Damage::PartialArchive => generation.join("frozen-input.json"),
        };
        let original = std::fs::read(&path)?;
        apply(damage, &path, &original)?;
        let damaged = readback::bundle_bytes(generation)?;
        let pointer = std::fs::read_link(case.root.join("current"))?;
        verify_refusal(publication::verify_published(target), damage)?;
        match boundary {
            Boundary::BeforeSwitch => {
                check!(eq; publication::current_workbook(&case.root)?, case.old)
            }
            Boundary::AfterSwitch => {
                verify_refusal(publication::current_workbook(&case.root), damage)?
            }
        }
        let result = publication::recover_publication(store, &case.root);
        match result {
            Err(ReportError::Publication(publication::PublicationError::RecoveryRequired {
                root,
                retained,
                detail,
            })) => {
                check!(eq; root, case.root);
                check!(eq; retained, vec![intent.request.expected.clone(), Root::Bundle(intent.desired().ok_or("desired")?.clone())]);
                check!(detail.contains(expected_detail(damage)));
            }
            other => {
                return Err(format!("damaged selected recovery did not refuse: {other:?}").into())
            }
        }
        check!(eq; harness::active_intent(store, &case.root)?, *intent);
        check!(eq; readback::bundle_bytes(generation)?, damaged);
        check!(eq; std::fs::read_link(case.root.join("current"))?, pointer);
        readback::old_unchanged(case)?;
        std::fs::write(&path, &original)?;
        readback::selected_bundle(target, &case.lineage, &case.witness)?;
    }
    Ok(())
}

fn apply(damage: Damage, path: &Path, original: &[u8]) -> TestResult {
    match damage {
        Damage::MissingProof => std::fs::remove_file(path)?,
        Damage::ChangedProof => {
            let mut changed = original.to_vec();
            *changed.first_mut().ok_or("nonempty selected proof")? ^= 1;
            std::fs::write(path, changed)?;
        }
        Damage::PartialArchive => {
            let partial = original.get(..original.len() / 2).ok_or("archive prefix")?;
            std::fs::write(path, partial)?;
        }
    }
    Ok(())
}

fn expected_detail(damage: Damage) -> &'static str {
    match damage {
        Damage::MissingProof => {
            "selected proof directory differs from its authenticated object inventory"
        }
        Damage::ChangedProof => "the frozen object does not hash to its name",
        Damage::PartialArchive => "generation artifact mismatch: frozen-input.json",
    }
}

fn verify_refusal<T>(result: crate::report::ReportResult<T>, damage: Damage) -> TestResult {
    let detail = match result {
        Err(ReportError::Invariant { detail })
            if matches!(damage, Damage::MissingProof | Damage::PartialArchive) =>
        {
            detail
        }
        Err(ReportError::ArtifactCorrupt { detail, .. })
            if matches!(damage, Damage::ChangedProof) =>
        {
            detail
        }
        Err(other) => {
            return Err(format!("unexpected selected verification refusal: {other}").into())
        }
        Ok(_) => return Err("partial selected bundle was accepted".into()),
    };
    check!(eq; detail, expected_detail(damage));
    Ok(())
}
