use crate::adblock::{AdblockController, AdblockState};
use crate::config::Config;
use crate::vpn::{VpnController, VpnState};
use anyhow::Result;

pub struct SentinelApp {
    pub config: Config,
    pub vpn: VpnController,
    pub adblock: AdblockController,
}

pub struct Status {
    pub vpn: VpnState,
    pub adblock: AdblockState,
}

impl SentinelApp {
    pub fn new(config: Config) -> Result<Self> {
        let vpn = VpnController::new(config.vpn.clone());
        let adblock = AdblockController::new(config.clone());
        Ok(Self {
            config,
            vpn,
            adblock,
        })
    }

    pub fn status(&self) -> Status {
        Status {
            vpn: self.vpn.status(),
            adblock: self.adblock.status(),
        }
    }
}
