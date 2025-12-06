mod adblock;
mod app;
mod config;
mod logging;
mod ui;
mod vpn;

use anyhow::Result;
use app::SentinelApp;
use config::Config;

fn main() -> Result<()> {
    logging::init();
    let config = Config::default();
    let mut app = SentinelApp::new(config)?;
    ui::run(&mut app)?;
    Ok(())
}
