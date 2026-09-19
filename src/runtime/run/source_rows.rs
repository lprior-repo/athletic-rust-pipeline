use super::super::{
    import::SourceManifest,
    row_protocol::RowJob,
    run_protocol::{RunRequest, MAX_RUN_ROWS},
    Runtime,
};
use crate::runtime::rankings_collection::RankingCollectionRef;
use crate::domain::identity::{SourceRowKey, WorkbookDigest};
use anyhow::{bail, Context, Result};
use std::collections::{BTreeSet, VecDeque};

const SOURCE_PAGE_ROWS: usize = 64;
const MAX_SHEETS: usize = 256;

struct SheetCursor {
    name: String,
    selected: u64,
    issued: u64,
    after_row: u32,
    pending: VecDeque<SourceRowKey>,
}

/// Ephemeral iterator only. Restate journals calls and completion order; this is
/// reconstructed from the immutable import on replay, never a checkpoint owner.
pub(super) struct SourceRows {
    workbook: WorkbookDigest,
    sheets: Vec<SheetCursor>,
    next_sheet: usize,
    pub selected: u64,
    rankings: Option<crate::runtime::rankings_collection::RankingCollectionRef>,
}
impl SourceRows {
    pub fn new(manifest: &SourceManifest, request: &RunRequest, rankings: Option<crate::runtime::rankings_collection::RankingCollectionRef>) -> Result<Self> {
        if manifest.stats.sheets.is_empty() || manifest.stats.sheets.len() > MAX_SHEETS {
            bail!("invalid source sheet count");
        }
        let mut names = BTreeSet::new();
        let total =
            manifest
                .stats
                .sheets
                .iter()
                .try_fold(0_u64, |total, sheet| -> Result<u64> {
                    if !names.insert(&sheet.name) {
                        bail!("duplicate source worksheet");
                    }
                    total
                        .checked_add(sheet.actual_data_rows)
                        .context("source count overflow")
                })?;
        if total == 0 || total != manifest.stats.actual_data_rows || total > MAX_RUN_ROWS {
            bail!("invalid manifest row accounting");
        }
        let sheets = manifest
            .stats
            .sheets
            .iter()
            .map(|sheet| SheetCursor {
                name: sheet.name.clone(),
                selected: request.selection.count(sheet.actual_data_rows),
                issued: 0,
                after_row: 1,
                pending: VecDeque::new(),
            })
            .collect::<Vec<_>>();
        let selected = sheets
            .iter()
            .try_fold(0_u64, |total, sheet| total.checked_add(sheet.selected))
            .context("selection count overflow")?;
        Ok(Self {
            workbook: manifest.workbook.clone(),
            sheets,
            next_sheet: 0,
            selected,
            rankings,
        })
    }

    pub fn update_rankings(&mut self, rankings: RankingCollectionRef) {
        self.rankings = Some(rankings);
    }
    pub async fn next(
        &mut self,
        runtime: &Runtime,
        request: &RunRequest,
    ) -> Result<Option<RowJob>> {
        for _ in 0..self.sheets.len() {
            let index = self.next_sheet;
            self.next_sheet = if index + 1 == self.sheets.len() {
                0
            } else {
                index + 1
            };
            let sheet = self
                .sheets
                .get_mut(index)
                .context("source cursor index is invalid")?;
            if sheet.issued == sheet.selected {
                continue;
            }
            if sheet.pending.is_empty() {
                refill(runtime, &self.workbook, sheet).await?;
            }
            let source = sheet
                .pending
                .pop_front()
                .context("source index exhausted before manifest count")?;
            sheet.issued = sheet
                .issued
                .checked_add(1)
                .context("issued source count overflow")?;
            return Ok(Some(RowJob {
                workbook: self.workbook.clone(),
                snapshot: request.snapshot.clone(),
                source,
                rankings: self.rankings.clone(),
            }));
        }
        Ok(None)
    }
}

async fn refill(
    runtime: &Runtime,
    workbook: &WorkbookDigest,
    sheet: &mut SheetCursor,
) -> Result<()> {
    let remaining = sheet
        .selected
        .checked_sub(sheet.issued)
        .context("source selection underflow")?;
    let limit = usize::try_from(remaining.min(SOURCE_PAGE_ROWS as u64))?;
    let store = runtime.store.clone();
    let workbook = workbook.clone();
    let name = sheet.name.clone();
    let after = sheet.after_row;
    let page = runtime
        .blocking(move || Ok(store.source_key_page(&workbook, &name, after, limit)?))
        .await?;
    let last = page
        .last()
        .context("source index exhausted before manifest count")?;
    sheet.after_row = last.row();
    sheet.pending = page.into();
    Ok(())
}
