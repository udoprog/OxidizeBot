use crate::{command, config, module};

pub struct Handler {}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, _: command::Context<'_, '_>) -> Result<(), failure::Error> {
        Ok(())
    }
}

pub struct Countdown {}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {}

impl Countdown {
    pub fn load(_config: &config::Config, _module: &Config) -> Result<Self, failure::Error> {
        Ok(Countdown {})
    }
}

impl super::Module for Countdown {
    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext { handlers, .. }: module::HookContext<'_>,
    ) -> Result<(), failure::Error> {
        handlers.insert("countdown", Handler {});
        Ok(())
    }
}
