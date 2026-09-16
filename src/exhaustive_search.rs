pub mod search {
    use crate::{
        config::Config,
        discovery::{AthleticNetClient, SearchFailure, SearchRequest},
        model::SearchHit,
        search_cache::{append, load_latest, SearchCacheRecord},
    };
    use anyhow::{Context, Result};
    use futures::{stream, StreamExt};
    use std::{collections::HashMap, path::Path};
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum StageError {
        #[error("{reason}")]
        Search {
            reason: String,
            partial_hits: Vec<SearchHit>,
        },
        #[error("{0}")]
        Storage(#[from] anyhow::Error),
    }

    pub struct Discovery {
        client: AthleticNetClient,
        cache: HashMap<String, SearchCacheRecord>,
        out_dir: std::path::PathBuf,
    }

    impl Discovery {
        pub fn new(config: &Config, out_dir: &Path) -> Result<Self> {
            let client = AthleticNetClient::new(&config.discovery)
                .context("initializing athletic.net client")?;
            let cache =
                load_latest(&out_dir.join("search-cache.jsonl")).context("loading search cache")?;
            Ok(Self {
                client,
                cache,
                out_dir: out_dir.to_owned(),
            })
        }

        pub async fn stage(
            &mut self,
            requests: &[SearchRequest],
        ) -> Result<Vec<SearchHit>, StageError> {
            let mut current_hits = Vec::new();
            let misses = requests
                .iter()
                .filter_map(|request| {
                    let key = request.cache_key();
                    match self.cache.get(&key) {
                        Some(record) if record.is_success() => {
                            current_hits.extend(record.hits.clone());
                            None
                        }
                        _ => Some((request.clone(), key)),
                    }
                })
                .collect::<Vec<_>>();
            let mut outcomes = {
                let client = &self.client;
                stream::iter(misses.into_iter().enumerate())
                    .map(|(index, (request, key))| async move {
                        let outcome = client.execute_exhaustive(&request).await;
                        (index, key, request, outcome)
                    })
                    .buffer_unordered(2)
                    .collect::<Vec<_>>()
                    .await
            };
            outcomes.sort_by_key(|(index, _, _, _)| *index);
            let mut search_failure = None;
            for (_, key, request, outcome) in outcomes {
                match outcome {
                    Ok(execution) => {
                        self.commit_success(key, &request, execution, &mut current_hits)?;
                    }
                    Err(failure) => {
                        if search_failure.is_none() {
                            search_failure = Some(failure.message.clone());
                        }
                        self.commit_failure(key, &request, &failure)?;
                    }
                }
            }
            match search_failure {
                Some(reason) => Err(StageError::Search {
                    reason,
                    partial_hits: current_hits,
                }),
                None => Ok(current_hits),
            }
        }

        fn commit_success(
            &mut self,
            key: String,
            request: &SearchRequest,
            execution: crate::discovery::SearchExecution,
            current_hits: &mut Vec<SearchHit>,
        ) -> Result<(), StageError> {
            let record = SearchCacheRecord::success(
                key.clone(),
                request.query.clone(),
                request.filter.clone(),
                execution.attempts,
                execution.hits.clone(),
            );
            append(&self.out_dir.join("search-cache.jsonl"), &record)
                .map_err(StageError::Storage)?;
            self.cache.insert(key, record);
            current_hits.extend(execution.hits);
            Ok(())
        }

        fn commit_failure(
            &mut self,
            key: String,
            request: &SearchRequest,
            failure: &SearchFailure,
        ) -> Result<(), StageError> {
            let error_msg = failure.message.clone();
            let mut record = SearchCacheRecord::retryable(
                key.clone(),
                request.query.clone(),
                request.filter.clone(),
                failure.attempts,
                error_msg,
            );
            record.retryable = failure.retryable;
            append(&self.out_dir.join("search-cache.jsonl"), &record)
                .map_err(StageError::Storage)?;
            self.cache.insert(key, record);
            Ok(())
        }
    }
}
