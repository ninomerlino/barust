use crate::{
    utils::{HookSender, TimedHooks},
    widget_default,
    widgets::{Rectangle, Result, Text, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use ipgeolocate::{GeoError, Locator, Service};
use log::debug;
use open_meteo_api::models::TimeZone;
use std::fmt::Debug;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug)]
pub struct Meteo {
    pub code: f32,
    pub city: String,
    pub current: String,
    pub max: String,
    pub min: String,
}

#[derive(Debug)]
pub struct OpenMeteoProvider;

impl OpenMeteoProvider {
    pub fn new() -> Box<Self> {
        Box::new(Self)
    }
}

#[async_trait]
impl WeatherProvider for OpenMeteoProvider {
    async fn get_current_meteo(&self) -> Result<Meteo> {
        let addr = public_ip::addr_v4().await.ok_or(Error::PublicIpNotFound)?;
        debug!("Reading current public ip:{}", addr);
        let loc_info = Locator::get(&addr.to_string(), Service::IpApi)
            .await
            .map_err(Error::from)?;

        let data = open_meteo_api::query::OpenMeteo::new()
            .coordinates(
                loc_info.latitude.parse::<f32>().unwrap(),
                loc_info.longitude.parse::<f32>().unwrap(),
            )
            .map_err(|e| Error::OpenMeteoRequest(e.to_string()))?
            .current_weather()
            .map_err(|e| Error::OpenMeteoRequest(e.to_string()))?
            .time_zone(TimeZone::Auto)
            .map_err(|e| Error::OpenMeteoRequest(e.to_string()))?
            .daily()
            .map_err(|e| Error::OpenMeteoRequest(e.to_string()))?
            .query()
            .await
            .map_err(|e| Error::OpenMeteoRequest(e.to_string()))?;

        let current_weather = data
            .current_weather
            .ok_or(Error::MissingData("current_weather"))?;
        let daily = data.daily.ok_or(Error::MissingData("daily"))?;
        let daily_units = data.daily_units.ok_or(Error::MissingData("daily_units"))?;

        let max = format!(
            "{}{}",
            daily
                .temperature_2m_max
                .first()
                .ok_or(Error::MissingData("max_temperature"))?
                .ok_or(Error::MissingData("max_temperature"))?,
            daily_units.temperature_2m_max
        );
        let min = format!(
            "{}{}",
            daily
                .temperature_2m_min
                .first()
                .ok_or(Error::MissingData("min_temperature"))?
                .ok_or(Error::MissingData("min_temperature"))?,
            daily_units.temperature_2m_min
        );
        let current = format!(
            "{}{}",
            current_weather.temperature, daily_units.temperature_2m_min
        );

        let out = Meteo {
            code: current_weather.weathercode,
            city: loc_info.city,
            current,
            max,
            min,
        };
        Ok(out)
    }
}

/// A set of strings used as icons in the Weather widget
#[derive(Debug)]
pub struct MeteoIcons {
    pub clear: String,
    pub cloudy: String,
    pub fog: String,
    pub freezing_rain: String,
    pub freezing_drizzle: String,
    pub hail: String,
    pub rain: String,
    pub snow: String,
    pub drizzle: String,
    pub light_snow: String,
    pub thunderstorm: String,
    pub unknown: String,
}

impl Default for MeteoIcons {
    fn default() -> Self {
        Self {
            clear: "󰖙".to_string(),
            cloudy: "󰖐".to_string(),
            drizzle: "󰖗".to_string(),
            fog: "󰖑".to_string(),
            freezing_drizzle: "󰖘".to_string(),
            freezing_rain: "󰙿".to_string(),
            hail: "󰖒".to_string(),
            light_snow: "󰖘".to_string(),
            rain: "󰖖".to_string(),
            snow: "󰼶".to_string(),
            thunderstorm: "".to_string(),
            unknown: "".to_string(),
        }
    }
}

impl MeteoIcons {
    /// Convert meteo code to icon
    fn translate_code(&self, value: u8) -> &str {
        match value {
            0 => &self.clear,
            1..=3 => &self.cloudy,
            45 | 48 => &self.fog,
            51 | 53 | 55 => &self.drizzle,
            56 | 57 => &self.freezing_drizzle,
            61 | 63 | 65 => &self.rain,
            66 | 67 => &self.freezing_rain,
            71 | 73 | 75 => &self.snow,
            77 => &self.light_snow,
            85 | 86 => &self.snow,
            95 => &self.thunderstorm,
            96 | 99 => &self.hail,
            _ => &self.unknown,
        }
    }
}

#[async_trait]
pub trait WeatherProvider: Send + std::fmt::Debug {
    async fn get_current_meteo(&self) -> Result<Meteo>;
}

/// Fetches and Displays the meteo at the current position using the machine public ip
#[derive(Debug)]
pub struct Weather {
    icons: MeteoIcons,
    format: String,
    inner: Text,
    provider: Box<dyn WeatherProvider>,
}

impl Weather {
    ///* `format`
    ///  * `%city` will be replaced with the current city used as reference for the meteo
    ///  * `%icon` will be replaced with the current symbol for the weather
    ///  * `%cur` will be replaced with the current temperature
    ///  * `%max` will be replaced with the max temperature
    ///  * `%min` will be replaced with the min temperature
    ///* `icons` a [&MeteoIcons]
    ///* `config` a [&WidgetConfig]
    pub async fn new(
        format: &impl ToString,
        icons: MeteoIcons,
        config: &WidgetConfig,
        provider: Box<impl WeatherProvider + 'static>,
    ) -> Box<Self> {
        Box::new(Self {
            icons,
            format: format.to_string(),
            inner: *Text::new("Loading...", config).await,
            provider,
        })
    }
}

#[async_trait]
impl Widget for Weather {
    async fn update(&mut self) -> Result<()> {
        debug!("updating meteo");
        let meteo = self.provider.get_current_meteo().await?;
        let text_str = self
            .format
            .replace("%city", &meteo.city.to_string())
            .replace("%icon", self.icons.translate_code(meteo.code as _))
            .replace("%cur", &meteo.current)
            .replace("%max", &meteo.max)
            .replace("%min", &meteo.min);
        self.inner.set_text(text_str);
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, _pool: &mut TimedHooks) -> Result<()> {
        // 1 hour
        tokio::spawn(async move {
            loop {
                if let Err(e) = sender.send().await {
                    debug!("breaking thread loop: {}", e);
                    break;
                }
                sleep(Duration::from_secs(3600)).await;
            }
        });
        Ok(())
    }

    widget_default!(draw, size, padding);
}

impl std::fmt::Display for Weather {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&String::from("Weather"), f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("Ip address not found")]
    PublicIpNotFound,
    Geo(#[from] GeoError),
    #[error("OpenMeteo request error: {0}")]
    OpenMeteoRequest(String),
    #[error("Missing data: {0}")]
    MissingData(&'static str),
}
