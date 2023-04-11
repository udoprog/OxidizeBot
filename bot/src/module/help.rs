use anyhow::Result;
use async_trait::async_trait;
use chat::command;
use chat::module;
use url::Url;

const DEFAULT_URL: &str = "https://setbac.tv/help";

/// Handler for the !help command.
pub(crate) struct Help {
    enabled: settings::Var<bool>,
    url: settings::Var<Url>,
}

#[async_trait]
impl command::Handler for Help {
    async fn handle(&self, ctx: &mut command::Context) -> Result<()> {
        if !self.enabled.load().await {
            return Ok(());
        }

        let next = ctx.next();
        let mut url = self.url.load().await;

        match next.as_deref() {
            None => {
                chat::respond!(
                    ctx,
                    "You can find documentation for each command at {}",
                    url
                );
            }
            Some(command) => {
                url.query_pairs_mut().append_pair("q", command);
                chat::respond!(ctx, format!("For help on that, go to {}", url));
            }
        }

        Ok(())
    }
}

pub(crate) struct Module;

#[async_trait]
impl chat::Module for Module {
    fn ty(&self) -> &'static str {
        "help"
    }

    async fn hook(
        &self,
        module::HookContext {
            handlers, settings, ..
        }: module::HookContext<'_>,
    ) -> Result<()> {
        let default_url = Url::parse(DEFAULT_URL)?;

        handlers.insert(
            "help",
            Help {
                enabled: settings.var("help/enabled", true).await?,
                url: settings.var("help/url", default_url).await?,
            },
        );

        Ok(())
    }
}
