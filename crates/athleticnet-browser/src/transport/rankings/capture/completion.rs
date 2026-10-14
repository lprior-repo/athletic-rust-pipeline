use super::super::rankings_helper::{
    click_numeric_page, validate_request, wait_for_active_page, CapturedRanking,
};
use super::CaptureContext;
use crate::protocol::RankingsCapture;
use crate::BrowserError;

pub(super) enum BindingDisposition {
    Discard,
    Emit,
    Paused,
}

pub(super) async fn complete_binding(
    parsed: &CapturedRanking,
    context: &CaptureContext<'_>,
    page_one_seen: &mut bool,
) -> Result<BindingDisposition, BrowserError> {
    if parsed.challenge {
        context.gate.revoke();
    }
    if context.action.capture == RankingsCapture::Navigation {
        if parsed.request_body.is_some() {
            return Ok(BindingDisposition::Discard);
        }
        return Ok(BindingDisposition::Emit);
    }
    let Some(request_page) = validate_request(
        parsed,
        context.action,
        context.action.page > 1 && !*page_one_seen,
    )?
    else {
        return Ok(BindingDisposition::Discard);
    };
    if request_page == context.action.page {
        return Ok(BindingDisposition::Emit);
    }
    click_through_first_page(context, request_page, page_one_seen).await?;
    Ok(BindingDisposition::Paused)
}

async fn click_through_first_page(
    context: &CaptureContext<'_>,
    request_page: u32,
    page_one_seen: &mut bool,
) -> Result<(), BrowserError> {
    if *page_one_seen || request_page != 1 {
        return Ok(());
    }
    *page_one_seen = true;
    wait_for_active_page(context.page, 1, context.deadline).await?;
    if !click_numeric_page(context.page, context.action.page).await? {
        return Err(BrowserError::Timeout);
    }
    Ok(())
}
