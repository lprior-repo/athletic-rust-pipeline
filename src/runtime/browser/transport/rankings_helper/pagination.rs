use super::capture::transport;
use crate::runtime::browser::BrowserError;
use chromiumoxide::{
    cdp::js_protocol::runtime::{RemoteObjectSubtype, RemoteObjectType},
    js::EvaluationResult,
    Page,
};
use futures::{StreamExt, TryStreamExt};
use std::time::Duration;
async fn active_page(page: &Page) -> Result<Option<u32>, BrowserError> {
    let result = transport(
        page.evaluate(
            r#"(() => {
                const pagination = document.querySelector('.pagination');
                if (!pagination) return null;
                const link = pagination.querySelector(
                    '.page-item.active .page-link, [aria-current="page"], [aria-current="page"] .page-link'
                );
                if (!link) return null;
                const text = link.textContent.trim();
                const value = Number(text);
                return Number.isInteger(value) && value > 0 ? value : null;
            })()"#,
        )
        .await,
        "active_page",
    )?;
    decode_page_value(result)
}

pub(super) fn decode_page_value(result: EvaluationResult) -> Result<Option<u32>, BrowserError> {
    let object = result.object();
    match (&object.r#type, &object.subtype) {
        (RemoteObjectType::Object, Some(RemoteObjectSubtype::Null)) if object.value.is_none() => {
            Ok(None)
        }
        (RemoteObjectType::Number, None) => {
            let page = result
                .into_value::<u32>()
                .map_err(|_| BrowserError::Protocol)?;
            if page == 0 {
                return Err(BrowserError::Protocol);
            }
            Ok(Some(page))
        }
        _ => Err(BrowserError::Protocol),
    }
}

const MAX_ACTIVE_PAGE_POLLS: usize = 256;

pub(crate) async fn wait_for_active_page(
    page: &Page,
    requested_page: u32,
    deadline: tokio::time::Instant,
) -> Result<u32, BrowserError> {
    let polls = futures::stream::iter(0..MAX_ACTIVE_PAGE_POLLS)
        .then(|_| async {
            match active_page(page).await? {
                Some(page_number) if page_number == requested_page => Ok(Some(page_number)),
                _ => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Ok(None)
                }
            }
        })
        .try_filter_map(|candidate| async move { Ok(candidate) });
    futures::pin_mut!(polls);
    tokio::time::timeout_at(deadline, polls.next())
        .await
        .map_err(|_| BrowserError::Timeout)?
        .transpose()?
        .ok_or(BrowserError::Timeout)
}

pub(crate) async fn click_numeric_page(
    page: &Page,
    requested_page: u32,
) -> Result<bool, BrowserError> {
    let js = format!(
        r#"(() => {{
            const pagination = document.querySelector('.pagination');
            if (!pagination) return false;
            const links = pagination.querySelectorAll('.page-link');
            for (const link of links) {{
                const text = link.textContent.trim();
                const value = Number(text);
                if (Number.isInteger(value) && value === {requested_page}) {{
                    const parent = link.closest('.page-item');
                    if (!parent || parent.classList.contains('disabled')
                        || link.getAttribute('aria-disabled') === 'true') return false;
                    link.click();
                    return true;
                }}
            }}
            return false;
        }})()"#,
        requested_page = requested_page,
    );
    transport(page.evaluate(&*js).await, "click_numeric_page")?
        .into_value::<bool>()
        .map_err(|_| BrowserError::Protocol)
}
