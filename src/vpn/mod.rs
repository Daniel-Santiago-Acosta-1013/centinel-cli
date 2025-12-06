mod dns;
mod dns_override;
mod forward;
mod pf;
mod routes;
mod tun_if;

use crate::config::Config;
use anyhow::{Result, bail};
use dns::{DnsServer, DnsServerHandle};
use dns_override::DnsOverride;
use forward::IpForwardGuard;
use log::{error, info};
use nix::unistd::Uid;
use pf::PfState;
use routes::{NetworkState, RouteManager};
use tun_if::{TunDevice, TunIp};

const NET_CIDR: &str = "10.99.0.0/24";
const TUN_HOST: &str = "10.99.0.1";
const TUN_PEER: &str = "10.99.0.2";
const DNS_LISTEN: &str = "127.0.0.1:5353";
const MTU: u16 = 1420;

#[derive(Clone, Debug)]
pub struct VpnTools {
    pub embedded: bool,
}

impl VpnTools {
    pub fn detect() -> Self {
        // Operamos sin binarios externos; dependemos de permisos de root.
        Self { embedded: true }
    }

    pub fn any(&self) -> bool {
        self.embedded
    }

    pub fn summary(&self) -> String {
        "embebida".to_string()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum VpnState {
    On,
    Off,
    Unknown,
}

impl VpnState {
    pub fn as_str(&self) -> &'static str {
        match self {
            VpnState::On => "encendida (local)",
            VpnState::Off => "apagada",
            VpnState::Unknown => "desconocido",
        }
    }
}

pub struct VpnController {
    config: Config,
    tools: VpnTools,
    state: VpnState,
    session: Option<VpnSession>,
}

struct VpnSession {
    tun: TunDevice,
    routes: RouteManager,
    pf: PfState,
    dns: DnsServerHandle,
    dns_override: DnsOverride,
    ip_forward: IpForwardGuard,
}

impl VpnController {
    pub fn new(config: Config, tools: VpnTools) -> Self {
        Self {
            config,
            tools,
            state: VpnState::Unknown,
            session: None,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        if !self.tools.any() {
            bail!("No hay soporte de VPN embebida disponible");
        }
        if !Uid::effective().is_root() {
            bail!(
                "Se requieren privilegios de administrador (sudo) para crear TUN, rutas, PF y DNS local."
            );
        }

        // Bloqueo: cargamos blocklist para DNS
        let blocklist = load_blocklist(self.config.blocklist_path.as_ref().map(|p| p.as_path()))?;

        // 1) Crear TUN
        let tun_ip = TunIp {
            host: TUN_HOST.parse()?,
            peer: TUN_PEER.parse()?,
            mtu: MTU,
        };
        let tun = TunDevice::create(tun_ip)?;

        // 2) Detectar red actual
        let net = NetworkState::detect()?;

        // 3) Rutas + NAT
        let mut routes = RouteManager::new(net.clone(), TUN_HOST.into());
        routes.apply()?;

        // 4) PF NAT
        let mut pf = PfState::new(net.primary_interface.clone(), NET_CIDR.into());
        pf.enable(&tun.name)?;

        // 5) DNS local con bloqueo
        let dns = DnsServer::start(DNS_LISTEN, blocklist.clone())?;

        // 6) Redirigir DNS del sistema a 127.0.0.1
        let mut dns_override = DnsOverride::new(net.primary_interface.clone())?;
        dns_override.apply_local()?;

        // 7) Habilitar forwarding IP en kernel
        let ip_forward = IpForwardGuard::enable()?;

        self.state = VpnState::On;
        self.session = Some(VpnSession {
            tun,
            routes,
            pf,
            dns,
            dns_override,
            ip_forward,
        });

        info!(
            "VPN local activa. Bloqueando {} dominios de anuncios.",
            blocklist.len()
        );
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut session) = self.session.take() {
            let mut first_err: Option<anyhow::Error> = None;

            if let Err(e) = session.dns.stop() {
                capture(&mut first_err, e);
            }
            if let Err(e) = session.dns_override.restore() {
                capture(&mut first_err, e);
            }
            if let Err(e) = session.ip_forward.restore() {
                capture(&mut first_err, e);
            }
            if let Err(e) = session.pf.disable() {
                capture(&mut first_err, e);
            }
            if let Err(e) = session.routes.remove() {
                capture(&mut first_err, e);
            }
            if let Err(e) = session.tun.down() {
                capture(&mut first_err, e);
            }

            if let Some(e) = first_err {
                self.state = VpnState::Off;
                return Err(e);
            }
        }

        self.state = VpnState::Off;
        Ok(())
    }

    pub fn status(&self) -> VpnState {
        self.state
    }
}

fn capture(slot: &mut Option<anyhow::Error>, err: anyhow::Error) {
    if slot.is_none() {
        *slot = Some(err);
    } else {
        error!("Error adicional al limpiar: {err}");
    }
}

fn load_blocklist(path: Option<&std::path::Path>) -> Result<Vec<String>> {
    use std::collections::HashSet;
    use std::fs;

    let mut set = HashSet::new();
    if let Some(p) = path {
        if p.exists() {
            let content = fs::read_to_string(p)?;
            for line in content.lines() {
                push_line(&mut set, line);
            }
        }
    }
    for line in include_str!("../../data/blocklist.txt").lines() {
        push_line(&mut set, line);
    }
    let mut v: Vec<String> = set.into_iter().collect();
    v.sort();
    Ok(v)
}

fn push_line(set: &mut std::collections::HashSet<String>, line: &str) {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') {
        return;
    }
    set.insert(s.to_lowercase());
}
