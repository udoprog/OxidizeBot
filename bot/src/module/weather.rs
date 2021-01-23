use crate::api::OpenWeatherMap;
use crate::auth;
use crate::command;
use crate::module;
use crate::prelude::*;
use anyhow::Result;
use uom::si::{
    f32::ThermodynamicTemperature,
    thermodynamic_temperature::{degree_celsius, degree_fahrenheit, kelvin},
    Unit as _,
};

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
enum TemperatureUnit {
    #[serde(rename = "degrees-celsius")]
    DegreesCelsius,
    #[serde(rename = "degrees-fahrenheit")]
    DegressFahrenheit,
    #[serde(rename = "kelvin")]
    Kelvin,
}

impl TemperatureUnit {
    /// Format the given temperature.
    pub fn with(self, t: ThermodynamicTemperature) -> String {
        match self {
            TemperatureUnit::DegreesCelsius => format!(
                "{:.1} {}",
                t.get::<degree_celsius>(),
                degree_celsius::abbreviation()
            ),
            TemperatureUnit::DegressFahrenheit => format!(
                "{:.1} {}",
                t.get::<degree_fahrenheit>(),
                degree_fahrenheit::abbreviation()
            ),
            TemperatureUnit::Kelvin => {
                format!("{:.1} {}", t.get::<kelvin>(), kelvin::abbreviation())
            }
        }
    }
}

/// Handler for the !weather command.
pub struct Weather {
    enabled: settings::Var<bool>,
    temperature_unit: settings::Var<TemperatureUnit>,
    location: settings::Var<Option<String>>,
    api: injector::Ref<OpenWeatherMap>,
}

#[async_trait]
impl command::Handler for Weather {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Weather)
    }

    async fn handle(&self, ctx: &mut command::Context) -> Result<()> {
        if !self.enabled.load().await {
            return Ok(());
        }

        match ctx.next().as_deref() {
            Some("current") => {
                let api = self.api.read().await;
                let api = api
                    .as_ref()
                    .ok_or_else(|| respond_err!("API not configured"))?
                    .clone();

                let loc = match ctx.rest() {
                    "" => self.location.load().await,
                    rest => Some(rest.to_string()),
                };

                let loc = match loc {
                    Some(loc) => loc,
                    None => {
                        respond!(ctx, "Must specify <location>");
                        return Ok(());
                    }
                };

                let temperature_unit = self.temperature_unit.load().await;

                let current = api.current(loc.clone()).await?;

                let current = match current {
                    Some(current) => current,
                    None => {
                        respond!(ctx, "Could not find location `{}`", loc);
                        return Ok(());
                    }
                };

                let mut parts = Vec::with_capacity(4);

                let t = ThermodynamicTemperature::new::<kelvin>(current.main.temp);

                parts.push(temperature_unit.with(t));

                for w in current.weather {
                    parts.push(w.to_string());
                }

                if let Some(rain) = current.rain {
                    parts.extend(match (rain._1h, rain._3h) {
                        (Some(m), _) => Some(format!("raining {:.0}mm/h", m)),
                        (_, Some(m)) => Some(format!("raining {:.0}mm/3h", m)),
                        _ => None,
                    });
                }

                if let Some(snow) = current.snow {
                    parts.extend(match (snow._1h, snow._3h) {
                        (Some(m), _) => Some(format!("snowing {:.0}mm/h", m)),
                        (_, Some(m)) => Some(format!("snowing {:.0}mm/3h", m)),
                        _ => None,
                    });
                }

                respond!(ctx, "{} -> {}.", current.name, parts.join(", "));
            }
            _ => {
                respond!(ctx, "Expected: current.");
            }
        }

        Ok(())
    }
}

pub struct Module;

#[async_trait]
impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "weather"
    }

    /// Set up command handlers for this module.
    async fn hook(
        &self,
        module::HookContext {
            handlers,
            settings,
            injector,
            ..
        }: module::HookContext<'_>,
    ) -> Result<()> {
        handlers.insert(
            "weather",
            Weather {
                enabled: settings.var("weather/enabled", false).await?,
                temperature_unit: settings
                    .var("weather/temperature-unit", TemperatureUnit::DegreesCelsius)
                    .await?,
                location: settings.optional("weather/location").await?,
                api: injector.var().await,
            },
        );

        Ok(())
    }
}
