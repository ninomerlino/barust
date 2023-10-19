use barust::{
    statusbar::StatusBar,
    utils::{Color, Position},
    widgets::{
        ActiveWindow, Battery, Brightness, Clock, Cpu, Disk, PulseaudioProvider, QtileWorkspaces,
        Spacer, SysfsProvider, Systray, Volume, WidgetConfig, Wlan,
    },
    Result,
};
use log::LevelFilter;
use std::{env, fs::OpenOptions, time::Duration};

const PURPLE: Color = Color::new(0.8, 0.0, 1.0, 1.0);
const BLANK: Color = Color::new(0.0, 0.0, 0.0, 0.0);

#[tokio::main]
async fn main() -> Result<()> {
    setup_logger();
    #[cfg(debug_assertions)]
    console_subscriber::init();

    let wd_config = WidgetConfig {
        font: "DejaVu Sans Mono".into(),
        font_size: 16.0,
        hide_timeout: Duration::from_secs(5),
        ..WidgetConfig::default()
    };

    StatusBar::create()
        .position(Position::Top)
        .background(BLANK)
        .left_widgets(vec![
            Spacer::new(20).await,
            QtileWorkspaces::new(
                PURPLE,
                10,
                &WidgetConfig {
                    padding: 0,
                    ..wd_config.clone()
                },
                &["scratchpad", "pulsemixer"],
                &(5..=9).map(|i| i.to_string()).collect::<Vec<_>>(),
            )
            .await,
            ActiveWindow::new(&WidgetConfig {
                flex: true,
                ..wd_config.clone()
            })
            .await?,
        ])
        .right_widgets(vec![
            Systray::new(20, &wd_config).await?,
            Disk::new("💾 %f", "/", &wd_config).await,
            Wlan::new("📡 %e", "wlp1s0".to_string(), &wd_config).await,
            Cpu::new("💻 %p%", &wd_config).await?,
            Battery::new("%i %c%", None, &wd_config).await?,
            Volume::new(
                "%i %p",
                Box::new(PulseaudioProvider::new().await.unwrap()),
                None,
                &wd_config,
            )
            .await,
            Brightness::new(
                "%i %p%",
                Box::new(SysfsProvider::new().await?),
                None,
                &wd_config,
            )
            .await,
            Clock::new("🕓 %H:%M %d/%m/%Y", &wd_config).await,
        ])
        .build()
        .await?
        .start()
        .await
}

fn setup_logger() {
    let args = env::args().collect::<Vec<_>>();

    let mut level = LevelFilter::Info;
    for arg in &args {
        level = match arg.as_str() {
            "--trace" => LevelFilter::Trace,
            "--debug" => LevelFilter::Debug,
            "--info" => LevelFilter::Info,
            "--warn" => LevelFilter::Warn,
            "--error" => LevelFilter::Error,
            _ => continue,
        }
    }

    if args.contains(&String::from("--stderr")) {
        simple_logging::log_to_stderr(level);
    } else {
        simple_logging::log_to(
            OpenOptions::new()
                .append(true)
                .open("/home/matteo/.local/share/barust.log")
                .unwrap(),
            level,
        );
        log_panics::Config::new()
            .backtrace_mode(log_panics::BacktraceMode::Resolved)
            .install_panic_hook();
    }
}
