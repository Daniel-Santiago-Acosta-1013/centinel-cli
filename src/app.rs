use crate::adblock::{AdblockController, AdblockState};
use crate::config::Config;
use crate::vpn::{VpnController, VpnState, VpnTools};
use anyhow::Result;

pub struct SentinelApp {
    pub vpn: VpnController,
    pub adblock: AdblockController,
    pub tools: VpnTools,
}

pub struct Status {
    pub vpn: VpnState,
    pub adblock: AdblockState,
}

impl SentinelApp {
    pub fn new(config: Config, tools: VpnTools) -> Result<Self> {
        let vpn = VpnController::new(config.clone(), tools.clone());
        let adblock = AdblockController::new(config.clone());
        Ok(Self {
            vpn,
            adblock,
            tools,
        })
    }

    pub fn status(&self) -> Status {
        Status {
            vpn: self.vpn.status(),
            adblock: self.adblock.status(),
        }
    }
}
