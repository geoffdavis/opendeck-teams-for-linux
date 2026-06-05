mod action;
mod mqtt;
mod settings;
mod state;

use openaction::{OpenActionResult, register_action, run};

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    if let Err(err) = simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Stderr,
        simplelog::ColorChoice::Never,
    ) {
        eprintln!("logger initialization failed: {err}");
    }

    let (controller, display_rx) = mqtt::MqttController::new();
    action::spawn_display_pusher(display_rx);
    register_action(action::ToggleMuteAction { controller }).await;

    run(std::env::args().collect()).await
}
