pub mod dns;
pub mod dns_override;
mod forward;
mod pf;
mod routes;
mod tun_if;

use crate::config::Config;
use anyhow::{Context, Result, bail};
use dns::{DnsServer, DnsServerHandle};
use dns_override::DnsOverride;
use forward::IpForwardGuard;
use log::{error, info};
use nix::unistd::Uid;
use pf::PfState;
use routes::{NetworkState, RouteManager};
use std::process::Command;
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
    tun: Option<TunDevice>,
    routes: Option<RouteManager>,
    pf: Option<PfState>,
    dns: Option<DnsServerHandle>,
    dns_override: Option<DnsOverride>,
    ip_forward: Option<IpForwardGuard>,
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

        // preflight: confirmamos que hoy tienes internet antes de tocar nada
        preflight_check()?;

        match self.start_full() {
            Ok(_) => {
                info!("VPN local activa en modo completo.");
                return Ok(());
            }
            Err(e) => {
                error!(
                    "Modo completo falló: {e}. Intentando modo solo-DNS para no dejarte sin internet."
                );
                let _ = self.stop(); // rollback best-effort
            }
        }

        self.start_dns_only()
    }

    fn start_full(&mut self) -> Result<()> {
        let blocklist = load_blocklist(self.config.blocklist_path.as_ref().map(|p| p.as_path()))?;

        // Sesión parcial para rollback si algo falla
        let mut session = VpnSession {
            tun: None,
            routes: None,
            pf: None,
            dns: None,
            dns_override: None,
            ip_forward: None,
        };

        // 1) Crear TUN
        let tun_ip = TunIp {
            host: TUN_HOST.parse()?,
            peer: TUN_PEER.parse()?,
            mtu: MTU,
        };
        let tun = TunDevice::create(tun_ip)?;
        let tun_name = tun.name().to_string();
        session.tun = Some(tun);

        // 2) Detectar red actual
        let net = NetworkState::detect()?;

        // 3) Rutas + NAT
        let mut routes = RouteManager::new(net.clone(), TUN_HOST.into());
        routes.apply()?;
        session.routes = Some(routes);

        // 4) PF NAT
        let mut pf = PfState::new(net.primary_interface.clone(), NET_CIDR.into());
        pf.enable(&tun_name)?;
        session.pf = Some(pf);

        // 5) DNS local con bloqueo
        let dns = DnsServer::start(DNS_LISTEN, blocklist.clone())?;
        session.dns = Some(dns);

        // 6) Redirigir DNS del sistema a 127.0.0.1
        let mut dns_override = DnsOverride::new(net.primary_interface.clone())?;
        dns_override.apply_local()?;
        session.dns_override = Some(dns_override);

        // 7) Habilitar forwarding IP en kernel
        let ip_forward = IpForwardGuard::enable()?;
        session.ip_forward = Some(ip_forward);

        // 8) Verificación rápida de conectividad y DNS
        health_check(&net, session.dns_override.as_ref())?;

        self.state = VpnState::On;
        self.session = Some(session);

        info!(
            "VPN local activa. Bloqueando {} dominios de anuncios.",
            blocklist.len()
        );
        Ok(())
    }

    fn start_dns_only(&mut self) -> Result<()> {
        let blocklist = load_blocklist(self.config.blocklist_path.as_ref().map(|p| p.as_path()))?;
        let net = NetworkState::detect()?;

        // DNS local con bloqueo
        let dns = DnsServer::start(DNS_LISTEN, blocklist)?;

        // Redirigir DNS del sistema a 127.0.0.1
        let mut dns_override = DnsOverride::new(net.primary_interface.clone())?;
        dns_override.apply_local()?;

        // Chequeo de conectividad (sin rutas ni NAT)
        health_check(&net, Some(&dns_override))?;

        self.state = VpnState::On;
        self.session = Some(VpnSession {
            tun: None,
            routes: None,
            pf: None,
            dns: Some(dns),
            dns_override: Some(dns_override),
            ip_forward: None,
        });

        info!("Modo respaldo: solo DNS + bloqueo de anuncios; no se tocaron rutas ni NAT.");
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut session) = self.session.take() {
            let mut first_err: Option<anyhow::Error> = None;

            if let Some(mut dns) = session.dns.take() {
                if let Err(e) = dns.stop() {
                    capture(&mut first_err, e);
                }
            }
            if let Some(mut dns_override) = session.dns_override.take() {
                if let Err(e) = dns_override.restore() {
                    capture(&mut first_err, e);
                }
            }
            if let Some(ipf) = session.ip_forward.take() {
                if let Err(e) = ipf.restore() {
                    capture(&mut first_err, e);
                }
            }
            if let Some(mut pf) = session.pf.take() {
                if let Err(e) = pf.disable() {
                    capture(&mut first_err, e);
                }
            }
            if let Some(mut routes) = session.routes.take() {
                if let Err(e) = routes.remove() {
                    capture(&mut first_err, e);
                }
            }
            if let Some(mut tun) = session.tun.take() {
                if let Err(e) = tun.down() {
                    capture(&mut first_err, e);
                }
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

pub fn load_blocklist(path: Option<&std::path::Path>) -> Result<Vec<String>> {
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

pub fn preflight_check() -> Result<()> {
    run_cmd(
        "ping",
        &["-c", "1", "-W", "1000", "1.1.1.1"],
        "Preflight: verificar internet actual (1.1.1.1)",
    )?;
    Ok(())
}

pub fn health_check(net: &NetworkState, dns_override: Option<&DnsOverride>) -> Result<()> {
    // Comprobamos que el gateway sigue alcanzable.
    run_cmd(
        "ping",
        &["-c", "1", "-W", "1000", &net.gateway],
        "Verificación de gateway",
    )?;
    // Verificamos internet saliendo
    run_cmd(
        "ping",
        &["-c", "1", "-W", "1000", "1.1.1.1"],
        "Verificación de internet (1.1.1.1)",
    )?;
    // Verificamos DNS resolviendo example.com
    run_cmd(
        "ping",
        &["-c", "1", "-W", "1000", "example.com"],
        "Verificación de DNS (example.com)",
    )?;

    // Verificamos que el DNS del sistema ahora apunta a 127.0.0.1 si aplicamos override
    if let Some(dns_override) = dns_override {
        let dns_out = Command::new("networksetup")
            .args(["-getdnsservers", dns_override.service()])
            .output()
            .context("networksetup -getdnsservers (health)")?;
        if !dns_out.status.success() {
            let stderr = String::from_utf8_lossy(&dns_out.stderr);
            anyhow::bail!("Health DNS: networksetup falló: {}", stderr);
        }
        let stdout = String::from_utf8_lossy(&dns_out.stdout);
        if !stdout.contains("127.0.0.1") {
            anyhow::bail!(
                "Health DNS: el DNS activo no es 127.0.0.1 en {}. Salida: {}",
                net.primary_interface,
                stdout
            );
        }
    }
    Ok(())
}

fn run_cmd(cmd: &str, args: &[&str], label: &str) -> Result<()> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .with_context(|| label.to_string())?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("{label} falló: {stderr}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::preflight_check;

    #[test]
    fn preflight_is_optional() {
        // Solo se ejecuta si ALLOW_NET_TEST=1 para no fallar en entornos sin red.
        if std::env::var("ALLOW_NET_TEST").ok().as_deref() != Some("1") {
            eprintln!("SKIP preflight_is_optional (set ALLOW_NET_TEST=1 to run real ping)");
            return;
        }
        preflight_check().expect("preflight debe pasar con internet disponible");
    }
}
