use super::{OptionCallback, Result, Text, Widget, WidgetConfig};
use log::debug;
use std::thread;
use xcb::{x::Window, Connection};

pub fn get_active_window_name(connection: &Connection) -> Result<String> {
    let ewmh_connection = xcb_wm::ewmh::Connection::connect(connection);
    let cookie = ewmh_connection.send_request(&xcb_wm::ewmh::proto::GetActiveWindow);
    let active_window_id: Window = ewmh_connection
        .wait_for_reply(cookie)
        .map_err(Error::from)?
        .window;
    let cookie = ewmh_connection.send_request(&xcb_wm::ewmh::proto::GetWmName(active_window_id));
    let active_window_name = ewmh_connection
        .wait_for_reply(cookie)
        .map_err(Error::from)?
        .name;

    Ok(active_window_name)
}

#[derive(Debug)]
pub struct ActiveWindow {
    inner: Text,
    on_click: OptionCallback<Self>,
}

impl ActiveWindow {
    pub fn new(config: &WidgetConfig, on_click: Option<fn(&mut Self)>) -> Box<Self> {
        Box::new(Self {
            inner: *Text::new("", config, None),
            on_click: on_click.into(),
        })
    }
}

impl Widget for ActiveWindow {
    fn draw(&self, context: &cairo::Context, rectangle: &cairo::Rectangle) -> Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn size(&self, context: &cairo::Context) -> Result<f64> {
        self.inner.size(context)
    }

    fn padding(&self) -> f64 {
        self.inner.padding()
    }

    fn hook(&mut self, sender: chan::Sender<()>) -> Result<()> {
        let (connection, screen_id) = Connection::connect(None).unwrap();
        let root_window = connection
            .get_setup()
            .roots()
            .nth(screen_id as usize)
            .unwrap()
            .root();
        connection
            .send_and_check_request(&xcb::x::ChangeWindowAttributes {
                window: root_window,
                value_list: &[xcb::x::Cw::EventMask(xcb::x::EventMask::PROPERTY_CHANGE)],
            })
            .map_err(Error::from)?;
        connection.flush().map_err(Error::from)?;
        thread::spawn(move || loop {
            if let Ok(xcb::Event::X(xcb::x::Event::PropertyNotify(_))) = connection.wait_for_event()
            {
                sender.send(());
            }
        });
        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        debug!("updating active_window");
        let (connection, _) = Connection::connect(None).map_err(Error::from)?;
        let window_name = get_active_window_name(&connection)?;
        self.inner.set_text(window_name);
        Ok(())
    }

    fn on_click(&mut self) {
        if let OptionCallback::Some(cb) = self.on_click {
            cb(self);
        }
    }
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Error {
    Xcb(xcb::Error),
}

impl From<xcb::ConnError> for Error {
    fn from(e: xcb::ConnError) -> Self {
        Error::Xcb(xcb::Error::Connection(e))
    }
}

impl From<xcb::ProtocolError> for Error {
    fn from(e: xcb::ProtocolError) -> Self {
        Error::Xcb(xcb::Error::Protocol(e))
    }
}
