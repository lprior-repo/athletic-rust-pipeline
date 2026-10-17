use super::super::rankings_helper::CapturedRanking;
use super::completion::{complete_binding, BindingDisposition};
use super::package::{binding_package, resolve_binding};
use super::{uniquely_bound, CaptureContext, PendingBinding, WireLedger};
use crate::transport::transfer::refuse_excess_wire;
use crate::BrowserError;
use chromiumoxide::cdp::js_protocol::runtime::EventBindingCalled;

pub(super) async fn process_binding_event(
    event: &EventBindingCalled,
    context: &CaptureContext<'_>,
    page_one_seen: &mut bool,
    ledger: &mut WireLedger,
) -> Result<Option<CapturedRanking>, BrowserError> {
    let Some(package) = binding_package(event, context) else {
        return Ok(None);
    };
    let request_url = package
        .get("requestUrl")
        .and_then(|value| value.as_str())
        .ok_or(BrowserError::Protocol)?
        .to_owned();
    let method = package
        .get("method")
        .and_then(|value| value.as_str())
        .ok_or(BrowserError::Protocol)?
        .to_owned();
    ledger.pending.push(PendingBinding {
        url: request_url,
        method,
        package,
    });
    try_resolve_pending(context, page_one_seen, ledger).await
}

pub(super) async fn try_resolve_pending(
    context: &CaptureContext<'_>,
    page_one_seen: &mut bool,
    ledger: &mut WireLedger,
) -> Result<Option<CapturedRanking>, BrowserError> {
    let mut index = 0;
    while index < ledger.pending.len() {
        let pending = ledger.pending.get(index).ok_or(BrowserError::Protocol)?;
        let pending_count = ledger.pending_count_for(&pending.url, &pending.method);
        let candidates = ledger.candidates_for(&pending.url, &pending.method);
        let Some(bound) = uniquely_bound(candidates, pending_count)? else {
            index = index.checked_add(1).ok_or(BrowserError::Protocol)?;
            continue;
        };
        if ledger.redirects.contains(&bound) {
            return Err(BrowserError::Redirect);
        }
        if ledger.failed.contains(&bound) {
            return Err(BrowserError::Transport);
        }
        let Some(total) = ledger.finished.get(&bound).copied() else {
            index = index.checked_add(1).ok_or(BrowserError::Protocol)?;
            continue;
        };
        refuse_excess_wire(Some(total))?;
        let pending = ledger.pending.remove(index);
        let resolved = resolve_binding(&pending.package, context)?;
        let Some(parsed) = resolved else {
            continue;
        };
        match complete_binding(&parsed, context, page_one_seen).await? {
            BindingDisposition::Discard => continue,
            BindingDisposition::Emit => return Ok(Some(parsed)),
            BindingDisposition::Paused => return Ok(None),
        }
    }
    Ok(None)
}
