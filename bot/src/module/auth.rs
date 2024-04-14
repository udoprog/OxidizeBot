use anyhow::Result;
use async_trait::async_trait;
use auth::TemporaryKind;
use chrono::Utc;
use common::Duration;

use chat::command;
use chat::module;

/// Handler for the !auth command.
pub(crate) struct Handler {
    auth: async_injector::Ref<auth::Auth>,
}

#[async_trait]
impl command::Handler for Handler {
    async fn handle(&self, ctx: &mut command::Context<'_>) -> Result<()> {
        let auth = self.auth.read().await;
        let auth = match auth.as_deref() {
            Some(auth) => auth,
            None => return Err(chat::respond_err!("auth component not configured").into()),
        };

        match ctx.next().as_deref() {
            Some("scopes") => {
                let filter = ctx.next();
                let filter = filter.as_deref();

                let user = match ctx.user.real() {
                    Some(user) => user,
                    None => {
                        chat::respond!(ctx, "Can only get scopes for real users");
                        return Ok(());
                    }
                };

                // apply the current filter to a collection of scopes.
                let filter = |list: Vec<auth::Scope>| {
                    list.into_iter()
                        .map(|s| s.to_string())
                        .filter(|s| filter.map(|f| s.contains(f)).unwrap_or(true))
                        .collect::<Vec<_>>()
                };

                let by_user = filter(auth.scopes_for_user(user.login()).await);

                let mut result = Vec::new();

                if !by_user.is_empty() {
                    result.push(format!(
                        "Your ({}): {}",
                        user.display_name(),
                        by_user.join(", ")
                    ));
                }

                for role in user.roles() {
                    let by_role = filter(auth.scopes_for_role(role).await);

                    if !by_role.is_empty() {
                        result.push(format!("{}: {}", role, by_role.join(", ")));
                    }
                }

                ctx.respond_lines(result, "*no scopes*").await;
            }
            Some("permit") | Some("grant") => {
                ctx.check_scope(auth::Scope::AuthPermit).await?;

                let duration: Duration = ctx.next_parse("<duration> <principal> <scope>")?;
                let principal = ctx.next_parse("<duration> <principal> <scope>")?;
                let scope = ctx.next_parse("<duration> <principal> <scope>")?;

                if !ctx.user.has_scope(scope).await {
                    chat::respond!(
                        ctx,
                        "Trying to grant scope `{}` that you don't have :(",
                        scope
                    );
                    return Ok(());
                }

                let now = Utc::now();
                let expires_at = now + duration.as_chrono();

                chat::respond!(
                    ctx,
                    "Gave: {scope} to {principal} for {duration}",
                    duration = duration,
                    principal = principal,
                    scope = scope
                );

                auth.insert_temporary(scope, principal, expires_at, TemporaryKind::Allow)
                    .await;
            }
            Some("deny") => {
                ctx.check_scope(auth::Scope::AuthPermit).await?;

                let duration: Duration = ctx.next_parse("<duration> <principal> <scope>")?;
                let principal = ctx.next_parse("<duration> <principal> <scope>")?;
                let scope = ctx.next_parse("<duration> <principal> <scope>")?;

                if !ctx.user.has_scope(scope).await {
                    chat::respond!(
                        ctx,
                        "Trying to deny scope `{}` that you don't have :(",
                        scope
                    );
                    return Ok(());
                }

                let now = Utc::now();
                let expires_at = now + duration.as_chrono();

                chat::respond!(
                    ctx,
                    "Denied: {scope} to {principal} for {duration}",
                    duration = duration,
                    principal = principal,
                    scope = scope
                );

                auth.insert_temporary(scope, principal, expires_at, TemporaryKind::Deny)
                    .await;
            }
            _ => {
                chat::respond!(ctx, "Expected: scopes, permit");
            }
        }

        Ok(())
    }
}

pub(crate) struct Module;

#[async_trait]
impl chat::Module for Module {
    fn ty(&self) -> &'static str {
        "auth"
    }

    async fn hook(
        &self,
        module::HookContext {
            injector, handlers, ..
        }: module::HookContext<'_, '_>,
    ) -> Result<()> {
        handlers.insert(
            "auth",
            Handler {
                auth: injector.var().await,
            },
        );
        Ok(())
    }
}
