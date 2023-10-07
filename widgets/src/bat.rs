use crate::{widget_default, Rectangle, Result, Text, Widget, WidgetConfig};
use async_trait::async_trait;
use cairo::Context;
use log::debug;
use std::{fmt::Display, fs::read_dir};
use utils::{percentage_to_index, HookSender, TimedHooks};

/// Icons used by [Battery]
#[derive(Debug)]
pub struct BatteryIcons {
    pub percentages: Vec<String>,
    ///displayed if the device is charging
    pub percentages_charging: Vec<String>,
}

impl Default for BatteryIcons {
    fn default() -> Self {
        let percentages = ['', '', '', '', '', '', '', '', '', '']
            .map(String::from)
            .to_vec();
        let percentages_charging = ['', '', '', '', '', '', '']
            .map(String::from)
            .to_vec();
        Self {
            percentages,
            percentages_charging,
        }
    }
}
/// Displays status and charge of the battery
#[derive(Debug)]
pub struct Battery {
    format: String,
    inner: Text,
    root_path: String,
    icons: BatteryIcons,
}

impl Battery {
    ///* `format`
    ///  * `%c` will be replaced with the charge percentage
    ///  * `%i` will be replaced with the correct icon from `icons`
    ///* `icons` sets a custom [BatteryIcons]
    ///* `config` a [&WidgetConfig]
    ///* `on_click` callback to run on click
    pub async fn new(
        format: impl ToString,
        icons: Option<BatteryIcons>,
        config: &WidgetConfig,
    ) -> Result<Box<Self>> {
        let mut root_path = String::default();
        for path in read_dir("/sys/class/power_supply")
            .map_err(Error::from)?
            .flatten()
        {
            let name = path.path().to_string_lossy().to_string();
            if name.contains("BAT") {
                root_path.clone_from(&name);
                break;
            }
        }
        if root_path.is_empty() {
            return Err(Error::NoBattery.into());
        }

        Ok(Box::new(Self {
            format: format.to_string(),
            inner: *Text::new("", config).await,
            root_path,
            icons: icons.unwrap_or_default(),
        }))
    }

    fn read_os_file(&self, filename: &str) -> Option<String> {
        let path = format!("{}/{}", self.root_path, filename);
        let value = std::fs::read_to_string(path).ok()?;
        Some(value.trim().into())
    }

    fn get_charge(&self) -> Option<f64> {
        self.percentage_from_files("charge_now", "charge_full")
    }

    fn get_energy(&self) -> Option<f64> {
        self.percentage_from_files("energy_now", "energy_full")
    }

    fn percentage_from_files(&self, f1: &str, f2: &str) -> Option<f64> {
        let v1 = self.read_os_file(f1)?.parse::<f64>().ok()?;
        let v2 = self.read_os_file(f2)?.parse::<f64>().ok()?;
        Some(v1 / v2 * 100.0)
    }
}

#[async_trait]
impl Widget for Battery {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    async fn update(&mut self) -> Result<()> {
        debug!("updating battery");
        let percent = match (self.get_charge(), self.get_energy()) {
            (Some(c), Some(_)) => c,
            (Some(c), None) => c,
            (None, Some(e)) => e,
            (None, None) => return Ok(()),
        };

        let percentages = if self.read_os_file("status") == Some("Charging".into()) {
            &self.icons.percentages_charging
        } else {
            &self.icons.percentages
        };

        let icon = {
            let percentages_len = percentages.len();
            let index = percentage_to_index(percent, (0, percentages_len - 1));
            &percentages[index]
        };

        let text = self
            .format
            .replace("%i", icon)
            .replace("%c", &percent.round().to_string());
        self.inner.set_text(text);
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks.subscribe(sender);
        Ok(())
    }

    widget_default!(size, padding);
}

impl Display for Battery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Battery").fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    IO(#[from] std::io::Error),
    #[error("No battery found")]
    NoBattery,
}
