//! Twitch API helpers.

use crate::api::RequestBuilder;
use crate::injector::Injector;
use crate::prelude::*;
use anyhow::Result;
use reqwest::{header, Client, Method, Url};
use std::fmt;
use std::sync::Arc;

const V2_URL: &str = "http://api.openweathermap.org/data/2.5";

/// API integration.
#[derive(Clone, Debug)]
pub(crate) struct OpenWeatherMap {
    client: Client,
    v2_url: Url,
    api_key: Arc<String>,
}

struct Builder {
    injector: Injector,
    pub(crate) api_key: Option<String>,
}

impl Builder {
    /// Inject a newly build value.
    pub(crate) async fn build_and_inject(&self) -> Result<()> {
        match &self.api_key {
            Some(api_key) => {
                self.injector
                    .update(OpenWeatherMap::new(api_key.to_string())?)
                    .await;
            }
            None => {
                let _ = self.injector.clear::<OpenWeatherMap>().await;
            }
        }

        Ok(())
    }
}

/// Hook up open weather api if all necessary settings are available.
pub(crate) async fn setup(
    settings: crate::Settings,
    injector: Injector,
) -> Result<impl Future<Output = Result<()>>> {
    let (mut api_key_stream, api_key) = settings
        .stream::<String>("weather/api-key")
        .optional()
        .await?;

    let mut builder = Builder { injector, api_key };

    builder.build_and_inject().await?;

    Ok(async move {
        loop {
            builder.api_key = api_key_stream.recv().await;
            builder.build_and_inject().await?;
        }
    })
}

impl OpenWeatherMap {
    /// Create a new API integration.
    pub(crate) fn new(api_key: String) -> Result<OpenWeatherMap> {
        Ok(OpenWeatherMap {
            client: Client::new(),
            v2_url: str::parse::<Url>(V2_URL)?,
            api_key: Arc::new(api_key),
        })
    }

    /// Build request against v2 URL.
    fn v2<I>(&self, method: Method, path: I) -> RequestBuilder<'_>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut url = self.v2_url.clone();

        if let Ok(mut p) = url.path_segments_mut() {
            p.extend(path);
        }

        let mut req = RequestBuilder::new(&self.client, method, url);
        req.query_param("appid", &*self.api_key)
            .header(header::ACCEPT, "application/json");
        req
    }

    pub(crate) async fn current(&self, q: String) -> Result<Option<Current>> {
        let mut req = self.v2(Method::GET, &["weather"]);
        req.query_param("q", &q);
        req.execute().await?.not_found().json()
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Current {
    pub(crate) coord: Coord,
    pub(crate) sys: Sys,
    pub(crate) weather: Vec<Weather>,
    pub(crate) main: Main,
    #[serde(default)]
    pub(crate) visibility: Option<u32>,
    pub(crate) wind: Wind,
    #[serde(default)]
    pub(crate) rain: Option<Precipitation>,
    #[serde(default)]
    pub(crate) snow: Option<Precipitation>,
    pub(crate) clouds: Clouds,
    pub(crate) dt: u64,
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) cod: u64,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Coord {
    pub(crate) lon: f32,
    pub(crate) lat: f32,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Sys {
    pub(crate) country: String,
    pub(crate) sunrise: u64,
    pub(crate) sunset: u64,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Weather {
    pub(crate) id: u64,
    pub(crate) main: String,
    pub(crate) description: String,
    pub(crate) icon: String,
}

impl fmt::Display for Weather {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.description)?;

        let icon = match self.icon.as_str() {
            "01d" => "☀️",
            "01n" => "🌑",
            "02d" => "⛅",
            "02n" | "03d" | "03n" => "☁️",
            "04d" | "04n" => "🌧️",
            "09d" | "09n" => "🌧️",
            "10d" | "10n" => "🌦️",
            "11d" | "11n" => "🌩️",
            "13d" | "13n" => "🌨️",
            _ => return Ok(()),
        };

        write!(fmt, " {}", icon)
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Main {
    pub(crate) temp: f32,
    #[serde(default)]
    pub(crate) humidity: Option<u64>,
    #[serde(default)]
    pub(crate) pressure: Option<u64>,
    #[serde(default)]
    pub(crate) temp_min: Option<f32>,
    #[serde(default)]
    pub(crate) temp_max: Option<f32>,
    #[serde(default)]
    pub(crate) sea_level: Option<u64>,
    #[serde(default)]
    pub(crate) grnd_level: Option<u64>,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Wind {
    #[serde(default)]
    pub(crate) speed: Option<f32>,
    #[serde(default)]
    pub(crate) deg: Option<f32>,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Precipitation {
    #[serde(rename = "1h", default)]
    pub(crate) _1h: Option<f32>,
    #[serde(rename = "3h", default)]
    pub(crate) _3h: Option<f32>,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Clouds {
    pub(crate) all: u64,
}
