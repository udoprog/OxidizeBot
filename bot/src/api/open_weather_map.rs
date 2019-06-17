//! Twitch API helpers.

use crate::{api::RequestBuilder, injector::Injector, prelude::*, settings::Settings};
use failure::{format_err, Error};
use reqwest::{header, r#async::Client, Method, StatusCode, Url};
use std::{fmt, sync::Arc};

const V2_URL: &'static str = "http://api.openweathermap.org/data/2.5";

/// API integration.
#[derive(Clone, Debug)]
pub struct OpenWeatherMap {
    client: Client,
    v2_url: Url,
    api_key: Arc<String>,
}

/// Hook up open weather api if all necessary settings are available.
pub fn setup(
    settings: Settings,
    injector: Injector,
) -> Result<impl Future<Output = Result<(), Error>>, Error> {
    let (mut api_key_stream, api_key) = settings.stream::<String>("weather/api-key").optional()?;

    let build = move |api_key: Option<String>| -> Result<(), Error> {
        match api_key {
            Some(api_key) => injector.update(OpenWeatherMap::new(api_key)?),
            None => injector.clear::<OpenWeatherMap>(),
        }

        Ok(())
    };

    Ok(async move {
        build(api_key)?;

        while let Some(update) = api_key_stream.next().await {
            build(update)?;
        }

        Err(format_err!("api-key stream ended"))
    })
}

impl OpenWeatherMap {
    /// Create a new API integration.
    pub fn new(api_key: String) -> Result<OpenWeatherMap, Error> {
        Ok(OpenWeatherMap {
            client: Client::new(),
            v2_url: str::parse::<Url>(V2_URL)?,
            api_key: Arc::new(api_key),
        })
    }

    /// Build request against v2 URL.
    fn v2(&self, method: Method, path: &[&str]) -> RequestBuilder {
        let mut url = self.v2_url.clone();

        {
            let mut url_path = url.path_segments_mut().expect("bad base");
            url_path.extend(path);
        }

        let req = RequestBuilder::new(self.client.clone(), method, url);
        let req = req.query_param("appid", &*self.api_key);
        req.header(header::ACCEPT, "application/json")
    }

    pub async fn current(&self, q: String) -> Result<Option<Current>, Error> {
        let data = self
            .v2(Method::GET, &["weather"])
            .query_param("q", &q)
            .json_option(not_found)
            .await?;
        Ok(data)
    }
}

/// Handle not found as a missing body.
fn not_found(status: &StatusCode) -> bool {
    match *status {
        StatusCode::NOT_FOUND => true,
        _ => false,
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Current {
    pub coord: Coord,
    pub sys: Sys,
    pub weather: Vec<Weather>,
    pub main: Main,
    #[serde(default)]
    pub visibility: Option<u32>,
    pub wind: Wind,
    #[serde(default)]
    pub rain: Option<Precipitation>,
    #[serde(default)]
    pub snow: Option<Precipitation>,
    pub clouds: Clouds,
    pub dt: u64,
    pub id: u64,
    pub name: String,
    pub cod: u64,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Coord {
    pub lon: f32,
    pub lat: f32,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Sys {
    pub country: String,
    pub sunrise: u64,
    pub sunset: u64,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Weather {
    pub id: u64,
    pub main: String,
    pub description: String,
    pub icon: String,
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
pub struct Main {
    pub temp: f32,
    #[serde(default)]
    pub humidity: Option<u64>,
    #[serde(default)]
    pub pressure: Option<u64>,
    #[serde(default)]
    pub temp_min: Option<f32>,
    #[serde(default)]
    pub temp_max: Option<f32>,
    #[serde(default)]
    pub sea_level: Option<u64>,
    #[serde(default)]
    pub grnd_level: Option<u64>,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Wind {
    #[serde(default)]
    pub speed: Option<f32>,
    #[serde(default)]
    pub deg: Option<f32>,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Precipitation {
    #[serde(rename = "1h", default)]
    pub _1h: Option<f32>,
    #[serde(rename = "3h", default)]
    pub _3h: Option<f32>,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Clouds {
    pub all: u64,
}
