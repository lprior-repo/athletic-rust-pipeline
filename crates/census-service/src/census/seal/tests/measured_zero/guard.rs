use super::fixture::{Fixture, Shape};
use super::{reached_phase, AcceptanceItem, Phase, SealEvidence, TestResult, Verification};

fn block(packet: &mut SealEvidence, item: AcceptanceItem) {
    match item {
        AcceptanceItem::JurisdictionSweepsTerminal => packet.open.jurisdiction_sweeps = None,
        AcceptanceItem::SourceObjectsTerminal => packet.open.source_objects = None,
        AcceptanceItem::CohortDecisionsTerminal => packet.open.cohort_decisions = None,
        AcceptanceItem::IdentityCandidatesTerminal => packet.open.identity_candidates = None,
        AcceptanceItem::ConflictsRetained => {
            packet.counts.athletes = 1;
            packet.retained.conflicts = 0;
        }
        AcceptanceItem::RetriesRepresented => packet.retained.access_conditions = 0,
        AcceptanceItem::EvidenceDurable => {
            packet.counts.athletes = 1;
            packet.retained.conflicts = 1;
            packet.retained.observations = 0;
        }
        AcceptanceItem::CalculationsReproducible => {
            packet.counts.cohort_performances = 1;
            packet.retained.calculations = 0;
        }
        AcceptanceItem::WorkbookMapped => {
            packet.counts.athletes = 1;
            packet.retained.conflicts = 1;
            packet.counts.class_of_2027 = 1;
            packet.workbook.mapped_athletes = 0;
        }
        AcceptanceItem::WorkbookCountsReconcile => {
            packet.workbook.counts_reconciled = Verification::NotMeasured;
        }
        AcceptanceItem::CoverageReportReconciles => {
            packet.workbook.coverage_reconciled = Verification::NotMeasured;
        }
        AcceptanceItem::RunMetricsReconcile => {
            packet.workbook.metrics_reconciled = Verification::NotMeasured;
        }
        AcceptanceItem::ExportVerified => {
            packet.workbook.export_verified = Verification::NotMeasured;
        }
        AcceptanceItem::RunBound => packet.run = None,
    }
}

#[test]
fn the_inspected_synthetic_journal_packet_advances_only_to_exporting() -> TestResult {
    for shape in [Shape::EmptyOptional, Shape::EmptyReview] {
        let fixture = Fixture::create(shape, true)?;
        fixture.assert_bundle()?;
        let packet = fixture.inspect()?;
        check!(packet.open_items().is_empty());
        let mut state = reached_phase(&fixture.store.stats()?, &fixture.path, &packet)?;
        check!(eq; state.phase(), Phase::Exporting);
        check!(
            state.sealed().is_none(),
            "phase inference never creates Complete"
        );
        state.seal(packet)?;
        check!(eq; state.phase(), Phase::Complete);
        check!(!fixture.store.out_dir().join("seal.json").exists());
    }
    Ok(())
}

#[test]
fn each_existing_acceptance_item_prevents_the_diagnostic_phase_lift() -> TestResult {
    for (shape, legacy_phase) in [
        (Shape::EmptyOptional, Phase::Acquiring),
        (Shape::EmptyReview, Phase::Reconciling),
    ] {
        let fixture = Fixture::create(shape, true)?;
        let inspected = fixture.inspect()?;
        check!(inspected.open_items().is_empty());
        let stats = fixture.store.stats()?;
        let sequence = fixture.sequence();
        let source_rows = fixture.source_rows()?;
        let publication = fixture.publication_bytes()?;
        for item in AcceptanceItem::ALL {
            let mut packet = inspected.clone();
            block(&mut packet, item);
            check!(eq; packet.open_items(), vec![item], "isolated unit predicate {item:?}");
            let mut state = reached_phase(&stats, &fixture.path, &packet)?;
            check!(eq; state.phase(), legacy_phase, "open item {item:?} keeps diagnostic inference");
            check!(state.seal(packet).is_err());
            check!(state.sealed().is_none());
        }
        check!(eq; fixture.sequence(), sequence);
        check!(eq; fixture.source_rows()?, source_rows);
        check!(eq; fixture.publication_bytes()?, publication);
        check!(!fixture.store.out_dir().join("seal.json").exists());
    }
    Ok(())
}

#[test]
fn export_discrepancies_prevent_lift_despite_all_verified_flags() -> TestResult {
    let fixture = Fixture::create(Shape::EmptyReview, true)?;
    let mut packet = fixture.inspect()?;
    packet
        .workbook
        .discrepancies
        .push("synthetic discrepancy predicate witness".into());
    check!(eq; packet.open_items(), vec![AcceptanceItem::ExportVerified]);
    let state = reached_phase(&fixture.store.stats()?, &fixture.path, &packet)?;
    check!(eq; state.phase(), Phase::Reconciling);
    check!(state.sealed().is_none());
    check!(!fixture.store.out_dir().join("seal.json").exists());
    Ok(())
}
