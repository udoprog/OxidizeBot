use crate::api::RequestBuilder;
use crate::prelude::BoxFuture;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::vec::IntoIter;

/// A Twitch category.
#[derive(Debug, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub box_art_url: String,
}

#[derive(Debug, Deserialize)]
pub struct Emote {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pagination {
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Page<T> {
    data: Vec<T>,
    #[serde(default)]
    pagination: Option<Pagination>,
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
pub struct Channel {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub game_name: Option<String>,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Badge {
    pub id: String,
    pub image_url_1x: String,
    pub image_url_2x: String,
    pub image_url_4x: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatBadge {
    pub set_id: String,
    pub versions: Vec<Badge>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ModifyChannelRequest<'a> {
    pub title: Option<&'a str>,
    pub game_id: Option<&'a str>,
}

/// A response that is paged as a stream of requests.
pub struct Paged<T> {
    /// The current page being returned.
    current: IntoIter<T>,
    /// Request template.
    request: RequestBuilder<'static>,
    /// Current future of a page to return.
    page: Option<BoxFuture<'static, Result<Page<T>>>>,
}

impl<T> Paged<T>
where
    T: serde::de::DeserializeOwned,
{
    /// Construct a new paged request.
    pub(crate) fn new(request: RequestBuilder<'_>) -> Self {
        let request = request.into_owned();
        let req = request.clone();
        let initial = async move { req.execute().await?.json::<Page<T>>() };

        Self {
            current: Vec::new().into_iter(),
            request,
            page: Some(Box::pin(initial)),
        }
    }
}

impl<T> futures_core::Stream for Paged<T>
where
    T: 'static + Unpin + Send + serde::de::DeserializeOwned,
{
    type Item = Result<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;

        loop {
            if let Some(item) = this.current.next() {
                cx.waker().wake_by_ref();
                return Poll::Ready(Some(Ok(item)));
            }

            let page = match &mut this.page {
                Some(page) => page,
                None => return Poll::Ready(None),
            };

            let page = match page.as_mut().poll(cx) {
                Poll::Ready(result) => {
                    this.page = None;
                    result?
                }
                Poll::Pending => return Poll::Pending,
            };

            let Page { data, pagination } = page;

            if data.is_empty() {
                return Poll::Ready(None);
            }

            if let Some(cursor) = pagination.and_then(|p| p.cursor) {
                let req = this.request.clone().query_param("after", &cursor);
                this.page = Some(Box::pin(async move { req.execute().await?.json() }));
            }

            this.current = data.into_iter();
        }
    }
}
