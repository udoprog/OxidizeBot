use anyhow::Result;
use async_trait::async_trait;
use chat::command;
use chat::module;

static MAGIC_8BALL_ANSWER: &[&str] = &[
    "It is certain.",
    "It is decidedly so.",
    "Without a doubt.",
    "Yes - definitely.",
    "You may rely on it.",
    "As I see it, yes.",
    "Most likely.",
    "Outlook good.",
    "Yes.",
    "Signs point to yes.",
    "Reply hazy, try again.",
    "Ask again later.",
    "Better not tell you now.",
    "Cannot predict now.",
    "Concentrate and ask again.",
    "Don't count on it.",
    "My reply is no.",
    "My sources say no.",
    "Outlook not so good.",
    "Very doubtful.",
];

/// Handler for the !8ball command.
pub(crate) struct EightBall {
    enabled: settings::Var<bool>,
}

#[async_trait]
impl command::Handler for EightBall {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::EightBall)
    }

    async fn handle(&self, ctx: &mut command::Context<'_>) -> Result<()> {
        use rand::Rng as _;

        if !self.enabled.load().await {
            return Ok(());
        }

        let rest = ctx.rest();

        if rest.trim().is_empty() {
            chat::respond!(ctx, "Ask a question.");
            return Ok(());
        }

        let index = rand::thread_rng().gen_range(0..MAGIC_8BALL_ANSWER.len());

        if let Some(answer) = MAGIC_8BALL_ANSWER.get(index) {
            chat::respond!(ctx, answer);
        }

        Ok(())
    }
}

pub(crate) struct Module;

#[async_trait]
impl chat::Module for Module {
    fn ty(&self) -> &'static str {
        "8ball"
    }

    /// Set up command handlers for this module.
    async fn hook(
        &self,
        module::HookContext {
            handlers, settings, ..
        }: module::HookContext<'_, '_>,
    ) -> Result<()> {
        handlers.insert(
            "8ball",
            EightBall {
                enabled: settings.var("8ball/enabled", true).await?,
            },
        );

        Ok(())
    }
}
