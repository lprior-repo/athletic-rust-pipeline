use super::*;
use crate::participation_evidence::result_ownership::{
    common_fixture, CheckedHtmlResultRowV1, CheckedHtmlResultSourceV1,
};
use census_domain::model::{SchoolId, StoreIdentity};
use census_store::{
    AttachmentContextObjectV1, JournalReadBudget, ResultSchoolCorrespondencePreparationV1,
    RetainedCaptureRootSchemaV1,
};

mod adverse;
mod commands;
mod fixture;
mod observe;

const GENUINE_BODY: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/sources/national-aggregators/samples/directathletics-tfrrs/indiana-tfrrs-list-5489.html"));

fn scope_for(index: usize) -> TestResult<ResultSchoolScopeV1> {
    let day = match index {
        0 => "2026-03-21",
        1 => "2026-03-11",
        _ => return Err("controlled TFRRS row index".into()),
    };
    Ok(ResultSchoolScopeV1 {
        day: ProfileDate::parse(day)?,
        sport: Sport::IndoorTrack,
        category: Gender::Boys,
        year: SchoolYear::from_date(day).ok_or("source school year")?,
    })
}

fn missing(detail: &str) -> crate::CrawlError {
    census_store::StoreError::Refused {
        detail: format!("controlled TFRRS school scenario: {detail}"),
    }
    .into()
}

#[test]
fn genuine_tfrrs_body_with_controlled_transport_and_cross_provider_school_review_survives_cold_replay(
) -> TestResult {
    let (directory, store, input) = fixture::install()?;
    let originals = fixture::originals(&store, &input)?;
    let mut approved = Vec::new();
    for index in 0..2 {
        let proposal = fixture::proposal(&store, &input, index, scope_for(index)?)?;
        fixture::verify_claim(&proposal, &input, index)?;
        approved.push(commands::accept(
            &store,
            &proposal,
            proposal.claim(),
            index,
        )?);
    }
    let before = setup::state(&store)?;
    let records = observe::current(&store)?;
    observe::verify(&records, &input)?;
    let proof = observe::freeze(&store)?;
    check!(eq; observe::frozen(&proof, &input)?, records);
    for (command, receipt) in &approved {
        let replay = review::replay(&store, command.clone())?;
        check!(replay.repeated());
        check!(eq; replay.receipt(), receipt);
    }
    check!(eq; fixture::originals(&store, &input)?, originals);
    check!(eq; setup::state(&store)?, before);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("store"))?.into_parts();
    drop(writer);
    check!(eq; setup::state(&store)?, before);
    check!(eq; fixture::originals(&store, &input)?, originals);
    check!(eq; observe::current(&store)?, records);
    check!(eq; observe::frozen(&proof, &input)?, records);
    for (command, receipt) in approved {
        let replay = review::replay(&store, command)?;
        check!(replay.repeated());
        check!(eq; replay.receipt(), &receipt);
    }
    check!(eq; setup::state(&store)?, before);
    Ok(())
}
