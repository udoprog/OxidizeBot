use crate::{auth, command, module, utils::Duration};
use chrono::Utc;
use failure::Error;

/// Handler for the !auth command.
pub struct Handler<'a> {
    auth: &'a auth::Auth,
}

impl<'a> command::Handler for Handler<'a> {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_>) -> Result<(), Error> {
        match ctx.next().as_ref().map(String::as_str) {
            Some("scopes") => {
                let filter = ctx.next();
                let filter = filter.as_ref().map(String::as_str);

                // apply the current filter to a collection of scopes.
                let filter = |list: Vec<auth::Scope>| {
                    list.into_iter()
                        .map(|s| s.to_string())
                        .filter(|s| filter.map(|f| s.contains(f)).unwrap_or(true))
                        .collect::<Vec<_>>()
                };

                let by_user = filter(self.auth.scopes_for_user(ctx.user.name));

                let mut result = Vec::new();

                if !by_user.is_empty() {
                    result.push(format!("Your ({}): {}", ctx.user.name, by_user.join(", ")));
                }

                for role in ctx.user.roles() {
                    let by_role = filter(self.auth.scopes_for_role(role));

                    if !by_role.is_empty() {
                        result.push(format!("{}: {}", role, by_role.join(", ")));
                    }
                }

                if result.is_empty() {
                    ctx.respond("*no scopes*");
                    return Ok(());
                }

                ctx.respond(format!("{}.", result.join("; ")));
            }
            Some("permit") => {
                ctx.check_scope(auth::Scope::AuthPermit)?;

                let duration: Duration = match ctx.next_parse("<duration> <principal> <scope>") {
                    Some(duration) => duration,
                    None => return Ok(()),
                };

                let principal = match ctx.next_parse("<duration> <principal> <scope>") {
                    Some(principal) => principal,
                    None => return Ok(()),
                };

                let scope = match ctx.next_parse("<duration> <principal> <scope>") {
                    Some(scope) => scope,
                    None => return Ok(()),
                };

                if !ctx.user.has_scope(scope) {
                    ctx.respond(format!(
                        "Trying to grant scope `{}` that you don't have :(",
                        scope
                    ));
                    return Ok(());
                }

                let now = Utc::now();
                let expires_at = now + duration.as_chrono();

                ctx.respond(format!(
                    "Gave: {scope} to {principal} for {duration}",
                    duration = duration,
                    principal = principal,
                    scope = scope
                ));

                self.auth.insert_temporary(scope, principal, expires_at);
            }
            _ => {
                ctx.respond("Expected: permit");
            }
        }

        Ok(())
    }
}

pub struct Module;

impl Module {
    pub fn load() -> Self {
        Module
    }
}

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "auth"
    }

    fn hook(
        &self,
        module::HookContext { handlers, auth, .. }: module::HookContext<'_, '_>,
    ) -> Result<(), Error> {
        handlers.insert("auth", Handler { auth });
        Ok(())
    }
}
