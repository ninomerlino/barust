use crate::utils::{bytes_to_closest, HookSender, TimedHooks};
use crate::{
    widget_default,
    widgets::{Rectangle, Result, Text, Widget, WidgetConfig},
};
use async_trait::async_trait;
use cairo::Context;
use std::fmt::Display;

#[derive(Debug)]
pub struct Disk {
    format: String,
    path: String,
    inner: Text,
}

impl Disk {
    ///* `format`
    ///  * *%p* will be replaced with the disk used percent
    ///  * *%u* will be replaced with the used disk
    ///  * *%f* will be replaced with the free disk
    ///  * *%t* will be replaced with the total disk
    ///* `config` a [&WidgetConfig]
    pub async fn new(
        format: impl ToString,
        path: impl ToString,
        config: &WidgetConfig,
    ) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            path: path.to_string(),
            inner: *Text::new("", config).await,
        })
    }
}

#[async_trait]
impl Widget for Disk {
    async fn update(&mut self) -> Result<()> {
        let disk_usage = psutil::disk::disk_usage(self.path.clone()).map_err(Error::from)?;
        let text = self
            .format
            .replace("%p", &disk_usage.percent().to_string())
            .replace("%u", &bytes_to_closest(disk_usage.used()))
            .replace("%f", &bytes_to_closest(disk_usage.free()))
            .replace("%t", &bytes_to_closest(disk_usage.total()));
        self.inner.set_text(text);
        Ok(())
    }

    async fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks.subscribe(sender);
        Ok(())
    }

    widget_default!(draw, size, padding);
}

impl Display for Disk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Disk").fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    Psutil(#[from] psutil::Error),
}
