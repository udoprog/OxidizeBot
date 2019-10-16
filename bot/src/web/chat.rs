use crate::{bus, message_log, web::EMPTY};
use std::sync::Arc;
use warp::{filters, path, Filter as _};

#[derive(serde::Deserialize)]
struct CommandQuery {
    command: String,
}

/// Chat endpoint.
#[derive(Clone)]
pub struct Chat {
    bus: Arc<bus::Bus<bus::Command>>,
    message_log: message_log::MessageLog,
}

impl Chat {
    pub fn route(
        bus: Arc<bus::Bus<bus::Command>>,
        message_log: message_log::MessageLog,
    ) -> filters::BoxedFilter<(impl warp::Reply,)> {
        let api = Self { bus, message_log };

        let command = warp::get2()
            .and(warp::path("command").and(warp::query::<CommandQuery>()))
            .and_then({
                let api = api.clone();
                move |query: CommandQuery| {
                    let api = api.clone();
                    async move { api.command(query).map_err(warp::reject::custom) }
                }
            })
            .boxed();

        let messages = warp::get2()
            .and(warp::path("messages").and(path::end()))
            .and_then({
                let api = api.clone();
                move || {
                    let api = api.clone();
                    async move { api.messages().map_err(warp::reject::custom) }
                }
            })
            .boxed();

        return warp::path("chat").and(command.or(messages)).boxed();
    }

    /// Run a command.
    fn command(&self, query: CommandQuery) -> Result<impl warp::Reply, anyhow::Error> {
        self.bus.send(bus::Command::Raw {
            command: query.command,
        });

        Ok(warp::reply::json(&EMPTY))
    }

    /// Get all stored messages.
    fn messages(&self) -> Result<impl warp::Reply, anyhow::Error> {
        let messages = self.message_log.messages();
        Ok(warp::reply::json(&*messages))
    }
}
