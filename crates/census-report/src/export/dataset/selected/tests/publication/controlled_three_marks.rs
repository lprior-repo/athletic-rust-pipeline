mod attachment;
pub(super) mod calendar_lifecycle;
mod calendar_source;
mod current_school_identity;
mod current_school_review;
mod current_school_source;
mod documentary_lifecycle;
mod documentary_material;
mod documentary_review;
mod flow;
mod objects;
mod point_source;
mod producer;
pub(super) mod proof_refusal;
mod publish;
mod readback;
mod school_authority;
pub(super) mod school_issuance;
mod school_review;
mod selection;
mod source;
mod window_intake;
mod window_refusals;
mod window_source;

fn reopen(directory: &tempfile::TempDir) -> source::TestResult<census_store::Store> {
    let (store, writer) =
        census_store::Store::open_public_identity_qualification_v3(directory.path().join("store"))?
            .into_parts();
    drop(writer);
    Ok(store)
}
