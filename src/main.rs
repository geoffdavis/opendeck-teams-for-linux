mod action;
mod control;
mod mqtt;
mod settings;
mod state;

use openaction::{OpenActionResult, run};

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

    action::register_controls().await;

    run(std::env::args().collect()).await
}
