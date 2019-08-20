use crate::{api::OpenWeatherMap, auth, command, module, prelude::*};
use failure::Error;
use parking_lot::RwLock;
use std::sync::Arc;
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
    enabled: Arc<RwLock<bool>>,
    temperature_unit: Arc<RwLock<TemperatureUnit>>,
    location: Arc<RwLock<Option<String>>>,
    api: Arc<RwLock<Option<OpenWeatherMap>>>,
}

#[async_trait]
impl command::Handler for Weather {
    fn scope(&self) -> Option<auth::Scope> {
        Some(auth::Scope::Weather)
    }

    async fn handle(&mut self, mut ctx: command::Context<'_>) -> Result<(), Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        match ctx.next().as_ref().map(String::as_str) {
            Some("current") => {
                let api = match self.api.read().clone() {
                    Some(api) => api,
                    None => {
                        ctx.respond("API not configured");
                        return Ok(());
                    }
                };

                let loc = match ctx.rest() {
                    "" => None,
                    rest => Some(rest.to_string()),
                };

                let loc = match loc.or_else(|| self.location.read().clone()) {
                    Some(loc) => loc,
                    None => {
                        ctx.respond("Must specify <location>");
                        return Ok(());
                    }
                };

                let user = ctx.user.clone();
                let user2 = user.clone();
                let temperature_unit = *self.temperature_unit.read();

                let future = async move {
                    let current = api.current(loc.clone()).await?;

                    let current = match current {
                        Some(current) => current,
                        None => {
                            user.respond(format!("Could not find location `{}`", loc));
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

                    user.respond(format!("{} -> {}.", current.name, parts.join(", ")));
                    Ok::<(), Error>(())
                };

                ctx.spawn(async move {
                    match future.await {
                        Ok(()) => (),
                        Err(e) => {
                            user2.respond("Failed to get current weather");
                            log_err!(e, "Failed to get current weather");
                        }
                    }
                });
            }
            _ => {
                ctx.respond("Expected: current.");
            }
        }

        Ok(())
    }
}

pub struct Module;

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "weather"
    }

    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext {
            handlers,
            settings,
            injector,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), Error> {
        handlers.insert(
            "weather",
            Weather {
                enabled: settings.var("weather/enabled", false)?,
                temperature_unit: settings
                    .var("weather/temperature-unit", TemperatureUnit::DegreesCelsius)?,
                location: settings.optional("weather/location")?,
                api: injector.var()?,
            },
        );

        Ok(())
    }
}
