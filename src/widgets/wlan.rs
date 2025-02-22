use crate::utils::{HookSender, TimedHooks};
use crate::{
    widget_default,
    widgets::{Rectangle, Result, Text, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use log::debug;
use std::fmt::Display;

/// Displays informations about a network interface
#[derive(Debug)]
pub struct Wlan {
    format: String,
    interface: String,
    inner: Text,
}

impl Wlan {
    ///* `format`
    ///  * `%i` will be replaced with the interface name
    ///  * `%e` will be replaced with the essid
    ///  * `%q` will be replaced with the signal quality
    ///* `interface` name of the network interface
    ///* `fg_color` foreground color
    pub async fn new(format: impl ToString, interface: String, config: &WidgetConfig) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            interface,
            inner: *Text::new("", config).await,
        })
    }

    fn build_string(&self) -> String {
        let Some(data) = iwlib::get_wireless_info(self.interface.clone()) else {
            return String::from("No interface");
        };
        self.format
            .replace("%i", &self.interface)
            .replace("%e", &data.wi_essid)
            .replace("%q", &data.wi_quality.to_string())
    }
}

#[async_trait]
impl Widget for Wlan {
    async fn update(&mut self) -> Result<()> {
        debug!("updating wlan");
        let text = self.build_string();
        self.inner.set_text(text);
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks.subscribe(sender);
        Ok(())
    }

    widget_default!(draw, size, padding);
}

impl Display for Wlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Network").fmt(f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {}
