mod blocklist;
mod hosts;

use crate::config::Config;
use anyhow::Result;
use blocklist::Blocklist;
use hosts::HostsManager;

#[derive(Clone, Copy, Debug)]
pub enum AdblockState {
    On,
    Off,
    Unknown,
}

impl AdblockState {
    pub fn as_str(&self) -> &'static str {
        match self {
            AdblockState::On => "activo",
            AdblockState::Off => "inactivo",
            AdblockState::Unknown => "desconocido",
        }
    }
}

pub struct AdblockController {
    blocklist: Blocklist,
    hosts: HostsManager,
    state: AdblockState,
}

impl AdblockController {
    pub fn new(config: Config) -> Self {
        let blocklist = Blocklist::new(config.blocklist_path.clone());
        let hosts = HostsManager::new(config.hosts_path.clone(), config.backup_path.clone());
        Self {
            blocklist,
            hosts,
            state: AdblockState::Unknown,
        }
    }

    pub fn enable(&mut self) -> Result<()> {
        let entries = self.blocklist.load()?;
        self.hosts.apply(&entries)?;
        self.state = AdblockState::On;
        Ok(())
    }

    pub fn disable(&mut self) -> Result<()> {
        self.hosts.remove()?;
        self.state = AdblockState::Off;
        Ok(())
    }

    pub fn status(&self) -> AdblockState {
        self.state
    }
}
