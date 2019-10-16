use crate::{command, module, prelude::*};
use anyhow::Error;
use parking_lot::RwLock;
use std::sync::Arc;
use url::Url;

const DEFAULT_URL: &str = "https://setbac.tv/help";

/// Handler for the !help command.
pub struct Help {
    enabled: Arc<RwLock<bool>>,
    url: Arc<RwLock<Url>>,
}

#[async_trait]
impl command::Handler for Help {
    async fn handle(&mut self, mut ctx: command::Context<'_>) -> Result<(), Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        let next = ctx.next();
        let mut url = self.url.read().clone();

        match next.as_ref().map(String::as_str) {
            None => {
                ctx.respond(format!(
                    "You can find documentation for each command at {}",
                    url
                ));
            }
            Some(command) => {
                url.query_pairs_mut().append_pair("q", command);
                ctx.respond(format!("For help on that, go to {}", url));
            }
        }

        Ok(())
    }
}

pub struct Module;

#[async_trait]
impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "help"
    }

    async fn hook(
        &self,
        module::HookContext {
            handlers, settings, ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), Error> {
        let default_url = Url::parse(DEFAULT_URL)?;

        handlers.insert(
            "help",
            Help {
                enabled: settings.var("help/enabled", true)?,
                url: settings.var("help/url", default_url)?,
            },
        );

        Ok(())
    }
}
