use super::super::parse::{credentials_in_bundle, parse_token};
use super::super::{DIRECTORY_URL, TOKEN_SCOPE, TOKEN_URL};
use crate::net::{FetchOptions, FetchOutcome, Fetcher};
use crate::{AdapterContext, CrawlError, CrawlResult};
use url::Url;

mod discovery;

#[cfg(test)]
mod tests;

pub(in crate::arbiter) async fn mint_token(
    ctx: &AdapterContext<'_>,
    options: &super::Options,
) -> CrawlResult<String> {
    let bundle = acquire_bundle(
        ctx.fetcher,
        DIRECTORY_URL,
        &super::fetch_options(ctx, options, Vec::new()),
    )
    .await?;
    let Some((client_id, client_secret)) =
        credentials_in_bundle(crate::directory::acquisition::text(&bundle)?)
    else {
        return Err(CrawlError::Schema {
            url: bundle.url.clone(),
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
    parse_token(crate::directory::acquisition::text(&body)?, TOKEN_URL)
}

async fn acquire_bundle(
    fetcher: &Fetcher,
    entry_url: &str,
    fetch: &FetchOptions,
) -> CrawlResult<FetchOutcome> {
    let authority = Url::parse(entry_url)
        .map_err(|_| discovery::refuse(entry_url, "invalid stable directory entry URL"))?;
    let entry = fetcher.get(entry_url, fetch).await?;
    crate::directory::acquisition::text(&entry)?;
    let module = discovery::discover(&entry, &authority)?;
    let bundle = fetcher.get(module.as_str(), fetch).await?;
    discovery::validate_bundle(&bundle, &authority)?;
    Ok(bundle)
}
