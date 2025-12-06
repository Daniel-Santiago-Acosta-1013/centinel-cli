use anyhow::{Context, Result};
use log::info;
use std::process::Command;

#[derive(Clone)]
pub struct NetworkState {
    pub primary_interface: String,
    pub gateway: String,
}

impl NetworkState {
    pub fn detect() -> Result<Self> {
        let out = Command::new("route")
            .args(["-n", "get", "default"])
            .output()
            .context("route -n get default")?;
        if !out.status.success() {
            anyhow::bail!(
                "route -n get default falló: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut iface = None;
        let mut gw = None;
        for line in stdout.lines() {
            if line.trim_start().starts_with("interface:") {
                iface = line.split_whitespace().nth(1).map(|s| s.to_string());
            }
            if line.trim_start().starts_with("gateway:") {
                gw = line.split_whitespace().nth(1).map(|s| s.to_string());
            }
        }
        let iface =
            iface.ok_or_else(|| anyhow::anyhow!("No se pudo detectar interfaz por defecto"))?;
        let gw = gw.ok_or_else(|| anyhow::anyhow!("No se pudo detectar gateway por defecto"))?;
        Ok(NetworkState {
            primary_interface: iface,
            gateway: gw,
        })
    }
}

pub struct RouteManager {
    net: NetworkState,
    tun_ip: String,
}

impl RouteManager {
    pub fn new(net: NetworkState, tun_ip: String) -> Self {
        Self { net, tun_ip }
    }

    pub fn apply(&mut self) -> Result<()> {
        // Rutas divididas para no romper IPv4 default.
        run("route", &["-n", "add", "-net", "0.0.0.0/1", &self.tun_ip])?;
        run("route", &["-n", "add", "-net", "128.0.0.0/1", &self.tun_ip])?;
        // Evita loop con el gateway físico.
        run(
            "route",
            &[
                "-n",
                "add",
                "-host",
                &self.net.gateway,
                "-interface",
                &self.net.primary_interface,
            ],
        )?;
        info!(
            "Rutas aplicadas: default via {} y host {} por {}",
            self.tun_ip, self.net.gateway, self.net.primary_interface
        );
        Ok(())
    }

    pub fn remove(&mut self) -> Result<()> {
        // Best effort; ignorar errores.
        let _ = run_ignore("route", &["-n", "delete", "-net", "0.0.0.0/1"]);
        let _ = run_ignore("route", &["-n", "delete", "-net", "128.0.0.0/1"]);
        let _ = run_ignore(
            "route",
            &[
                "-n",
                "delete",
                "-host",
                &self.net.gateway,
                "-interface",
                &self.net.primary_interface,
            ],
        );
        info!("Rutas revertidas");
        Ok(())
    }
}

fn run(cmd: &str, args: &[&str]) -> Result<()> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .context(cmd.to_string())?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("{} {:?} falló: {}", cmd, args, stderr);
    }
    Ok(())
}

fn run_ignore(cmd: &str, args: &[&str]) -> Result<()> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .context(cmd.to_string())?;
    if !out.status.success() {
        return Ok(());
    }
    Ok(())
}
