use crate::{widget_default, Rectangle, Result, Text, Widget, WidgetConfig};
use async_trait::async_trait;
use cairo::Context;
use log::debug;
use std::{fmt::Display};
use utils::{percentage_to_index, HookSender, ResettableTimer, ReturnCallback, TimedHooks};

/// Icons used by [Volume]
#[derive(Debug)]
pub struct VolumeIcons {
    pub percentages: Vec<String>,
    ///displayed if the device is muted
    pub muted: String,
}

impl Default for VolumeIcons {
    fn default() -> Self {
        let percentages = ['奄', '奔', '墳'];
        Self {
            percentages: percentages.map(String::from).to_vec(),
            muted: String::from('ﱝ'),
        }
    }
}
/// Displays status and volume of the audio device
#[derive(Debug)]
pub struct Volume {
    format: String,
    inner: Text,
    volume_command: ReturnCallback<Option<f64>>,
    muted_command: ReturnCallback<Option<bool>>,
    icons: VolumeIcons,
    previous_volume: f64,
    previous_muted: bool,
    show_counter: ResettableTimer,
}

impl Volume {
    ///* `format`
    ///  * *%p* will be replaced with the volume percentage
    ///  * *%i* will be replaced with the correct icon
    ///* `volume_command` a function that returns the volume in a range from 0 to 100
    ///* `muted_command` a function that returns true if the volume is muted
    ///* `icons` sets a custom [VolumeIcons]
    ///* `config` a [&WidgetConfig]
    ///* `on_click` callback to run on click
    pub async fn new(
        format: impl ToString,
        volume_command: &'static (dyn Fn() -> Option<f64> + Send + Sync),
        muted_command: &'static (dyn Fn() -> Option<bool> + Send + Sync),
        icons: Option<VolumeIcons>,
        config: &WidgetConfig,
    ) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            volume_command: volume_command.into(),
            muted_command: muted_command.into(),
            icons: icons.unwrap_or_default(),
            previous_volume: 0.0,
            previous_muted: false,
            show_counter: ResettableTimer::new(config.hide_timeout),
            inner: *Text::new("", config).await,
        })
    }
}

#[async_trait]
impl Widget for Volume {
    fn draw(&self, context: &Context, rectangle: &Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating volume");
        let muted = self.muted_command.call().unwrap_or(false);
        let volume = self.volume_command.call().unwrap_or(0.0);

        if self.previous_muted != muted || self.previous_volume != volume {
            self.previous_muted = muted;
            self.previous_volume = volume;
            self.show_counter.reset();
        }
        let text = self.build_string(volume, muted);

        self.inner.set_text(text);
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks.subscribe(sender).map_err(Error::from)?;
        Ok(())
    }

    widget_default!(size, padding);
}

impl Volume {
    fn build_string(&mut self, volume: f64, muted: bool) -> String {
        if self.show_counter.is_done() {
            return String::from("");
        }
        if muted {
            return self.icons.muted.clone();
        }
        let percentages_len = self.icons.percentages.len();
        let index = percentage_to_index(volume, (0, percentages_len - 1));
        self.format
            .replace("%p", &format!("{:.1}", volume))
            .replace("%i", &self.icons.percentages[index].to_string())
    }
}

impl Display for Volume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Volume").fmt(f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    HookChannel(#[from] crossbeam_channel::SendError<HookSender>),
    Psutil(#[from] psutil::Error),
}
