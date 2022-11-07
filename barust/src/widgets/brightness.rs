use super::{OnClickCallback, Result, Text, Widget, WidgetConfig};
use crate::{
    corex::{Callback, EmptyCallback, HookSender, RawCallback, ResettableTimer, TimedHooks},
    forward_to_inner,
};
use std::{fmt::Display, time::Duration};

#[derive(Debug)]
pub struct Brightness {
    format: String,
    brightness_command: Callback<(), Option<i32>>,
    previous_brightness: i32,
    show_counter: ResettableTimer,
    inner: Text,
    on_click: OnClickCallback,
}

impl Brightness {
    pub fn new(
        format: impl ToString,
        brightness_command: &'static RawCallback<(), Option<i32>>,
        config: &WidgetConfig,
        on_click: Option<&'static EmptyCallback>,
    ) -> Box<Self> {
        Box::new(Self {
            format: format.to_string(),
            inner: *Text::new("", config, None),
            previous_brightness: 0,
            brightness_command: brightness_command.into(),
            on_click: on_click.map(|c| c.into()),
            show_counter: ResettableTimer::new(config.hide_timeout),
        })
    }
}

impl Widget for Brightness {
    fn draw(&self, context: &cairo::Context, rectangle: &cairo::Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn update(&mut self) -> Result<()> {
        let current_brightness = self
            .brightness_command
            .call(())
            .ok_or(Error::CommandError)?;

        if current_brightness != self.previous_brightness {
            self.previous_brightness = current_brightness;
            self.show_counter.reset();
        }

        let text = if self.show_counter.is_done() {
            String::from("")
        } else {
            self.format
                .replace("%b", &format!("{:.0}", current_brightness))
        };
        self.inner.set_text(text);

        Ok(())
    }

    fn hook(&mut self, sender: HookSender, timed_hooks: &mut TimedHooks) -> Result<()> {
        timed_hooks
            .subscribe(self.show_counter.duration / 10, sender)
            .map_err(Error::from)?;
        Ok(())
    }

    fn on_click(&self) {
        if let Some(cb) = &self.on_click {
            cb.call(());
        }
    }

    forward_to_inner!(size);
    forward_to_inner!(padding);
}

impl Display for Brightness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from("Brightness").fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    #[error("Failed to execute brightness command")]
    CommandError,
    HookChannel(#[from] crossbeam_channel::SendError<(Duration, HookSender)>),
}
