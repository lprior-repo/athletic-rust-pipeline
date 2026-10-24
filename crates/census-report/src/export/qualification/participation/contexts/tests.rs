use super::*;
use census_domain::model::SourceRef;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn aggregate_boundary_accepts_duplicate_but_refusal_preserves_result_state() -> TestResult {
    let athlete = AthleteId::mint("ath_subject", &["context"]);
    let added = u64::try_from(athlete.as_str().len() + 33)?;
    let mut context = ResultContexts {
        proofs: 15_999_999,
        bytes: 1_073_741_824 - added,
        ..Default::default()
    };
    context.retain(Entity::Athlete(&athlete), [0; 32])?;
    let before = context.entities.clone();
    context.retain(Entity::Athlete(&athlete), [0; 32])?;
    let other = AthleteId::mint("ath_subject", &["foreign"]);
    check!(matches!(
        context.retain(Entity::Athlete(&other), [0; 32]),
        Err(StoreError::BudgetExceeded {
            what: "verified result context proof count",
            needed: 16_000_001,
            available: 16_000_000
        })
    ));
    check!(context.entities == before);
    check!(eq; (context.proofs, context.bytes), (16_000_000, 1_073_741_824));
    Ok(())
}

#[test]
fn byte_refusal_does_not_insert_result_entity_or_charge_count() -> TestResult {
    let athlete = AthleteId::mint("ath_subject", &["context"]);
    let added = u64::try_from(athlete.as_str().len() + 33)?;
    let mut context = ResultContexts {
        proofs: 7,
        bytes: 1_073_741_825 - added,
        ..Default::default()
    };
    check!(matches!(
        context.retain(Entity::Athlete(&athlete), [0; 32]),
        Err(StoreError::BudgetExceeded {
            what: "verified result context encoded bytes",
            needed: 1_073_741_825,
            available: 1_073_741_824
        })
    ));
    check!(context.entities.is_empty());
    check!(eq; (context.proofs, context.bytes), (7, 1_073_741_825 - added));
    Ok(())
}

#[test]
fn distinct_memberships_cross_one_million_without_charging_duplicates() -> TestResult {
    let mut context = ResultContexts::default();
    for entity_ordinal in 0..128_u64 {
        let athlete = AthleteId::mint("ath_subject", &[&entity_ordinal.to_string()]);
        check!(eq; athlete.as_str().len(), 28);
        for ordinal in 0..8_192_u64 {
            context.retain(Entity::Athlete(&athlete), ordinal_digest(ordinal)?)?;
        }
        let proofs = context
            .entities
            .get(&Kind::Athlete)
            .and_then(|rows| rows.get(athlete.as_str()))
            .ok_or("missing populated athlete context")?;
        check!(eq; proofs.len(), 8_192);
        check!(proofs.contains(&ordinal_digest(0)?));
        check!(proofs.contains(&ordinal_digest(8_191)?));
        check!(!proofs.contains(&ordinal_digest(8_192)?));
        let counts = (context.proofs, context.bytes);
        context.retain(Entity::Athlete(&athlete), ordinal_digest(0)?)?;
        context.retain(Entity::Athlete(&athlete), ordinal_digest(8_191)?)?;
        check!(eq; (context.proofs, context.bytes), counts);
    }
    let rows = context
        .entities
        .get(&Kind::Athlete)
        .ok_or("missing athlete contexts")?;
    check!(eq; rows.len(), 128);
    check!(eq; context.entities.len(), 1);
    check!(eq; (context.proofs, context.bytes), (1_048_576, 63_963_136));
    Ok(())
}

#[test]
fn same_fingerprint_is_distinct_for_each_entity_id_and_kind() -> TestResult {
    let athlete = AthleteId::mint("ath_subject", &["context"]);
    let other = AthleteId::mint("ath_subject", &["other"]);
    let event: EventId = athlete.cast();
    check!(eq; athlete.as_str(), event.as_str());
    check!(athlete != other);
    let evidence = result_evidence(0);
    let digest = fingerprint(&evidence)?;
    let mut context = ResultContexts::default();
    context.retain(Entity::Athlete(&athlete), digest)?;
    check!(context.contains(Entity::Athlete(&athlete), &evidence)?);
    check!(!context.contains(Entity::Athlete(&other), &evidence)?);
    check!(!context.contains(Entity::Event(&event), &evidence)?);
    context.retain(Entity::Athlete(&other), digest)?;
    context.retain(Entity::Event(&event), digest)?;
    check!(context.contains(Entity::Athlete(&other), &evidence)?);
    check!(context.contains(Entity::Event(&event), &evidence)?);
    check!(eq; (context.proofs, context.bytes), (3, 183));
    let before = context.entities.clone();
    context.retain(Entity::Athlete(&athlete), digest)?;
    context.retain(Entity::Athlete(&other), digest)?;
    context.retain(Entity::Event(&event), digest)?;
    check!(context.entities == before);
    check!(eq; (context.proofs, context.bytes), (3, 183));
    Ok(())
}

#[test]
fn physical_entity_limit_preserves_result_state_and_duplicate_identity() -> TestResult {
    let athlete = AthleteId::mint("ath_subject", &["context"]);
    let mut context = ResultContexts::default();
    for ordinal in 0..65_536_u64 {
        context.retain(Entity::Athlete(&athlete), ordinal_digest(ordinal)?)?;
    }
    check!(eq; context.proofs, 65_536);
    let before = context.entities.clone();
    let counts = (context.proofs, context.bytes);
    context.retain(Entity::Athlete(&athlete), ordinal_digest(0)?)?;
    check!(matches!(
        context.retain(Entity::Athlete(&athlete), ordinal_digest(65_536)?),
        Err(StoreError::Refused { detail })
            if detail == "verified result context exceeds 65536 proofs per physical entity"
    ));
    check!(context.entities == before);
    check!(eq; (context.proofs, context.bytes), counts);
    Ok(())
}

#[test]
fn counter_overflow_does_not_mutate_result_state_or_charge() -> TestResult {
    let athlete = AthleteId::mint("ath_subject", &["overflow"]);
    for (proofs, bytes) in [(u64::MAX, 0), (7, u64::MAX)] {
        let mut context = ResultContexts {
            proofs,
            bytes,
            ..Default::default()
        };
        check!(matches!(
            context.retain(Entity::Athlete(&athlete), [0; 32]),
            Err(StoreError::CounterOverflow)
        ));
        check!(context.entities.is_empty());
        check!(eq; (context.proofs, context.bytes), (proofs, bytes));
    }
    Ok(())
}

#[test]
fn distinct_result_fingerprints_share_one_event_past_previous_limit() -> TestResult {
    let event = EventId::mint("event", &["shared-context"]);
    let mut context = ResultContexts::default();
    for ordinal in 0..257_u64 {
        let evidence = result_evidence(ordinal);
        context.retain(Entity::Event(&event), fingerprint(&evidence)?)?;
        check!(context.contains(Entity::Event(&event), &evidence)?);
    }
    check!(eq; context.proofs, 257);
    check!(context
        .entities
        .get(&Kind::Event)
        .and_then(|rows| rows.get(event.as_str()))
        .is_some_and(|proofs| proofs.len() == 257));
    check!(context.contains(Entity::Event(&event), &result_evidence(0))?);
    check!(context.contains(Entity::Event(&event), &result_evidence(256))?);
    check!(!context.contains(Entity::Event(&event), &result_evidence(257))?);
    let counts = (context.proofs, context.bytes);
    context.retain(Entity::Event(&event), fingerprint(&result_evidence(0))?)?;
    check!(eq; (context.proofs, context.bytes), counts);
    Ok(())
}

fn ordinal_digest(ordinal: u64) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let mut digest = [0; 32];
    digest
        .get_mut(..8)
        .ok_or("missing digest prefix")?
        .copy_from_slice(&ordinal.to_le_bytes());
    Ok(digest)
}

fn result_evidence(ordinal: u64) -> Evidence {
    let mut evidence = Evidence::parsed(SourceRef::id("milesplit"), "2026-10-22T00:00:00Z");
    evidence.note = Some(
        serde_json::json!({
            "result_id": ordinal,
            "locator": format!("result:{ordinal}"),
            "source_athlete": format!("athlete:{ordinal}")
        })
        .to_string(),
    );
    evidence
}
