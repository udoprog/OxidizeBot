use crate::auth;
use crate::command;
use crate::module;
use crate::prelude::*;
use crate::template::Template;
use anyhow::Result;
use chrono::prelude::*;
use chrono::Utc;
use chrono_tz::{Etc, Tz};

/// Handler for the !8ball command.
pub struct Time {
    enabled: settings::Var<bool>,
    timezone: settings::Var<Tz>,
    template: settings::Var<Template>,
}

#[async_trait]
impl command::Handler for Time {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Time)
    }

    async fn handle(&self, ctx: &mut command::Context) -> Result<()> {
        if !self.enabled.load().await {
            return Ok(());
        }

        let tz = self.timezone.load().await;
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

        let response = self.template.load().await.render_to_string(Vars {
            day: now.day(),
            month: now.month(),
            year: now.year(),
            offset: &offset,
            time: &time,
            rfc2822: &rfc2822,
        })?;

        respond!(ctx, response);
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

#[async_trait]
impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "time"
    }

    /// Set up command handlers for this module.
    async fn hook(
        &self,
        module::HookContext {
            handlers, settings, ..
        }: module::HookContext<'_>,
    ) -> Result<()> {
        let default_template = Template::compile("The streamer's time is {{time}}{{offset}}")?;

        handlers.insert(
            "time",
            Time {
                enabled: settings.var("time/enabled", true).await?,
                timezone: settings.var("time/timezone", Etc::UTC).await?,
                template: settings.var("time/template", default_template).await?,
            },
        );

        Ok(())
    }
}
