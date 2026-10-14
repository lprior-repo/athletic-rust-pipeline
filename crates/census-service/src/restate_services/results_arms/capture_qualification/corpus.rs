use census_domain::{model::SourceMeetRef, UsJurisdiction};
use census_store::{Store, Table};

use super::{
    config::{Profile, BATCH_ROWS},
    TestResult,
};

pub(super) fn seed(store: &Store, profile: &Profile) -> TestResult {
    let batch_rows = usize::try_from(BATCH_ROWS)?;
    let jurisdictions = u64::try_from(UsJurisdiction::CENSUS_SCOPE.len())?;
    (0..profile.rows).step_by(batch_rows).try_for_each(|start| {
        let end = start
            .checked_add(BATCH_ROWS)
            .ok_or("batch range overflow")?
            .min(profile.rows);
        let mut batch = Vec::new();
        batch.try_reserve_exact(usize::try_from(
            end.checked_sub(start).ok_or("batch count")?,
        )?)?;
        (start..end).try_for_each(|index| {
            let owner = usize::try_from(
                index
                    .checked_rem(jurisdictions)
                    .ok_or("jurisdiction divisor")?,
            )?;
            let jurisdiction = UsJurisdiction::CENSUS_SCOPE
                .get(owner)
                .copied()
                .ok_or("fixture jurisdiction")?;
            let row = row(profile, jurisdiction, index);
            let encoded = serde_json::to_vec(&row)?;
            check!(
                encoded.len()
                    <= profile
                        .payload
                        .len()
                        .checked_add(512)
                        .ok_or("encoded bound")?
            );
            batch.push(row);
            Ok::<_, Box<dyn std::error::Error>>(())
        })?;
        store.append_many(Table::SourceMeets, &batch)?;
        Ok::<_, Box<dyn std::error::Error>>(())
    })
}

pub(super) fn row(profile: &Profile, jurisdiction: UsJurisdiction, index: u64) -> SourceMeetRef {
    let id = format!("{}-{index:08}", jurisdiction.code());
    let date = match index % 100 {
        0 => Some("2024-04-01"),
        1 => None,
        _ => Some("2023-04-01"),
    };
    SourceMeetRef {
        id: SourceMeetRef::row_id("synthetic_source_capture", &id),
        source: "synthetic_source_capture".to_string(),
        source_meet_id: id.clone(),
        jurisdiction,
        season: "outdoor".to_string(),
        year: 2024,
        name: profile.payload.clone(),
        date: date.map(str::to_string),
        venue: "synthetic venue".to_string(),
        results_url: format!("https://fixture.invalid/{id}/results"),
        observed_on: "2026-10-08".to_string(),
    }
}
