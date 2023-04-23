use anyhow::{anyhow, Result};
use tokio::sync::RwLockReadGuard;
use warp::body;
use warp::filters;
use warp::path;
use warp::Filter;

use crate::EMPTY;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct DeleteRequest {
    ns: Option<serde_json::Value>,
    key: serde_json::Value,
}

/// Cache endpoints.
#[derive(Clone)]
pub(crate) struct Cache(async_injector::Ref<storage::Cache>);

impl Cache {
    pub(crate) fn route(
        cache: async_injector::Ref<storage::Cache>,
    ) -> filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Cache(cache);

        let list = warp::get()
            .and(path::end().and_then({
                let api = api.clone();
                move || {
                    let api = api.clone();
                    async move { api.list().await.map_err(super::custom_reject) }
                }
            }))
            .boxed();

        let delete = warp::delete()
            .and(path::end().and(body::json()).and_then({
                move |body: DeleteRequest| {
                    let api = api.clone();
                    async move { api.delete(body).await.map_err(super::custom_reject) }
                }
            }))
            .boxed();

        warp::path("cache").and(list.or(delete)).boxed()
    }

    /// Access underlying cache abstraction.
    async fn cache(&self) -> Result<RwLockReadGuard<'_, storage::Cache>> {
        match self.0.read().await {
            Some(out) => Ok(out),
            None => Err(anyhow!("cache not configured")),
        }
    }

    /// List all cache entries.
    async fn list(&self) -> Result<impl warp::Reply> {
        let entries = self.cache().await?.list_json()?;
        Ok(warp::reply::json(&entries))
    }

    /// Delete a cache entry.
    async fn delete(&self, request: DeleteRequest) -> Result<impl warp::Reply> {
        self.cache()
            .await?
            .delete_with_ns(request.ns.as_ref(), &request.key)?;
        Ok(warp::reply::json(&EMPTY))
    }
}
