use crate::{auth, command, module, prelude::*, template::Template};
use chrono::{
    offset::{Offset as _, TimeZone as _},
    Datelike as _, Timelike as _, Utc,
};
use chrono_tz::{Etc, Tz};
use failure::Error;
use parking_lot::RwLock;
use std::sync::Arc;

/// Handler for the !8ball command.
pub struct Time {
    enabled: Arc<RwLock<bool>>,
    timezone: Arc<RwLock<Tz>>,
    template: Arc<RwLock<Template>>,
}

#[async_trait]
impl command::Handler for Time {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Time)
    }

    async fn handle<'ctx>(&mut self, ctx: command::Context<'ctx>) -> Result<(), Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        let tz = self.timezone.read().clone();
        let now = Utc::now();

        let offset = tz.offset_from_utc_datetime(&now.naive_utc());
        let offset = offset.fix().local_minus_utc();
        let offset = format_time_zone(offset);

        let now = now.with_timezone(&tz);

        let time = now.time();
        let time = format!(
            "{:02}:{:02}:{:02}",
            time.hour(),
            time.minute(),
            time.second()
        );

        let rfc2822 = now.to_rfc2822();

        let response = self.template.read().render_to_string(Vars {
            day: now.day(),
            month: now.month(),
            year: now.year(),
            offset: &offset,
            time: &time,
            rfc2822: &rfc2822,
        })?;

        ctx.respond(response);
        return Ok(());

        #[derive(serde::Serialize)]
        struct Vars<'a> {
            day: u32,
            month: u32,
            year: i32,
            offset: &'a str,
            time: &'a str,
            rfc2822: &'a str,
        }

        /// Format the given offset as a timezone offset.
        fn format_time_zone(mut offset: i32) -> String {
            let mut neg = false;

            if offset < 0 {
                offset = -offset;
                neg = true;
            }

            let minutes = (offset % 3600) / 60;
            let hours = offset / 3600;

            if neg {
                format!("-{:02}{:02}", hours, minutes)
            } else {
                format!("+{:02}{:02}", hours, minutes)
            }
        }
    }
}

pub struct Module;

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "time"
    }

    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext {
            handlers,
            settings,
            futures,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), Error> {
        let mut vars = settings.vars();

        let default_template = Template::compile("The streamer's time is {{time}}{{offset}}")?;

        handlers.insert(
            "time",
            Time {
                enabled: vars.var("time/enabled", true)?,
                timezone: vars.var("time/timezone", Etc::UTC)?,
                template: vars.var("time/template", default_template)?,
            },
        );

        futures.push(vars.run().boxed());
        Ok(())
    }
}
