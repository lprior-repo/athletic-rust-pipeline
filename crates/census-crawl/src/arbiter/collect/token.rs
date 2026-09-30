use super::super::parse::{credentials_in_bundle, parse_token};
use super::super::{BUNDLE_URL, TOKEN_SCOPE, TOKEN_URL};
use crate::net::FetchOptions;
use crate::{AdapterContext, CrawlError, CrawlResult};

pub(in crate::arbiter) async fn mint_token(
    ctx: &AdapterContext<'_>,
    options: &super::Options,
) -> CrawlResult<String> {
    let bundle = ctx
        .fetcher
        .get(BUNDLE_URL, &fetch_options(ctx, options, Vec::new()))
        .await
        .map_err(|error| CrawlError::Invariant {
            detail: format!(
                "the Arbiter directory bundle {BUNDLE_URL} could not be read: {error}; the asset \
                 name is pinned and changes when Arbiter redeploys"
            ),
        })?;
    let Some((client_id, client_secret)) = credentials_in_bundle(&bundle.text()) else {
        return Err(CrawlError::Schema {
            url: BUNDLE_URL.to_string(),
            detail: "no client_id/client_secret pair in the published bundle".to_string(),
        });
    };
    let form = vec![
        ("client_id".to_string(), client_id),
        ("client_secret".to_string(), client_secret),
        ("grant_type".to_string(), "client_credentials".to_string()),
        ("scope".to_string(), TOKEN_SCOPE.to_string()),
    ];
    let mut token_fetch = ctx.fetch_options();
    token_fetch.refresh = true;
    let body = ctx
        .fetcher
        .post_form(TOKEN_URL, &form, &token_fetch)
        .await?;
    parse_token(&body.text(), TOKEN_URL)
}

fn fetch_options(
    ctx: &AdapterContext<'_>,
    options: &super::Options,
    headers: Vec<(String, String)>,
) -> FetchOptions {
    let mut fetch = ctx.fetch_options();
    fetch.refresh = options.refresh || ctx.refresh;
    fetch.headers = headers;
    fetch
}
