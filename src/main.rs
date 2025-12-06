mod adblock;
mod app;
mod config;
mod logging;
mod setup;
mod ui;
mod vpn;

use anyhow::Result;
use app::SentinelApp;
use config::Config;
use setup::ensure;

fn main() -> Result<()> {
    logging::init();
    let mut config = Config::default();
    let setup = ensure(&mut config)?;
    let mut app = SentinelApp::new(config, setup.tools.clone())?;
    ui::run(&mut app, &setup)?;
    Ok(())
}
