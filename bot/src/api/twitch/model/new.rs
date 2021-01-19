use crate::api::RequestBuilder;
use crate::prelude::BoxFuture;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pagination {
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Page<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub pagination: Option<Pagination>,
}

#[derive(Deserialize)]
pub struct Clip {
    pub id: String,
    pub edit_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: String,
    pub login: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub broadcaster_type: String,
    pub description: String,
    pub profile_image_url: String,
    pub offline_image_url: String,
    pub view_count: u64,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Subscription {
    pub broadcaster_id: String,
    pub broadcaster_name: String,
    pub is_gift: bool,
    pub tier: String,
    pub plan_name: String,
    pub user_id: String,
    pub user_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Stream {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    #[serde(default)]
    pub game_id: Option<String>,
    #[serde(default)]
    pub community_ids: Vec<String>,
    #[serde(rename = "type")]
    pub ty: String,
    pub title: String,
    pub viewer_count: u64,
    pub started_at: DateTime<Utc>,
    pub language: String,
    pub thumbnail_url: String,
}

/// A response that is paged as a stream of requests.
pub struct Paged<T> {
    pub(crate) request: RequestBuilder,
    pub(crate) page: Option<BoxFuture<'static, Result<Page<T>>>>,
}

impl<T> futures_core::Stream for Paged<T>
where
    T: 'static + Send + serde::de::DeserializeOwned,
{
    type Item = Result<Vec<T>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;

        let page = match &mut this.page {
            Some(page) => page,
            None => return Poll::Ready(None),
        };

        let result = match page.as_mut().poll(cx) {
            Poll::Ready(result) => result,
            Poll::Pending => return Poll::Pending,
        };

        this.page = None;

        let page = match result {
            Ok(page) => page,
            Err(e) => return Poll::Ready(Some(Err(e))),
        };

        let Page { data, pagination } = page;

        if data.is_empty() {
            return Poll::Ready(None);
        }

        if let Some(cursor) = pagination.and_then(|p| p.cursor) {
            let req = this.request.clone().query_param("after", &cursor);
            this.page = Some(Box::pin(async move { req.execute().await?.json() }));
        }

        Poll::Ready(Some(Ok(data)))
    }
}
