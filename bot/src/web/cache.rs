use crate::web::EMPTY;
use warp::{body, filters, path, Filter as _};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct DeleteRequest {
    ns: Option<serde_json::Value>,
    key: serde_json::Value,
}

/// Cache endpoints.
#[derive(Clone)]
pub struct Cache(crate::storage::Cache);

impl Cache {
    pub fn route(cache: crate::storage::Cache) -> filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Cache(cache);

        let list = warp::get2()
            .and(path::end().and_then({
                let api = api.clone();
                move || api.list().map_err(warp::reject::custom)
            }))
            .boxed();

        let delete = warp::delete2()
            .and(path::end().and(body::json()).and_then({
                let api = api.clone();
                move |body: DeleteRequest| api.delete(body).map_err(warp::reject::custom)
            }))
            .boxed();

        return warp::path("cache").and(list.or(delete)).boxed();
    }

    /// List all cache entries.
    fn list(&self) -> Result<impl warp::Reply, failure::Error> {
        let entries = self.0.list_json()?;
        Ok(warp::reply::json(&entries))
    }

    /// Delete a cache entry.
    fn delete(&self, request: DeleteRequest) -> Result<impl warp::Reply, failure::Error> {
        self.0.delete_with_ns(request.ns.as_ref(), request.key)?;
        Ok(warp::reply::json(&EMPTY))
    }
}
