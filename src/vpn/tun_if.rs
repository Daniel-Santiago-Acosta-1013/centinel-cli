use anyhow::{Context, Result};
use log::info;
use std::net::Ipv4Addr;
use std::process::Command;
use tun::{AbstractDevice, Configuration, Device, create};

pub struct TunDevice {
    pub name: String,
    _dev: Device,
}

pub struct TunIp {
    pub host: Ipv4Addr,
    pub peer: Ipv4Addr,
    pub mtu: u16,
}

impl TunDevice {
    pub fn create(ip: TunIp) -> Result<Self> {
        let mut config = Configuration::default();
        config
            .address(ip.host)
            .destination(ip.peer)
            .netmask(Ipv4Addr::new(255, 255, 255, 0))
            .up()
            .mtu(ip.mtu);

        let dev = create(&config).context("No se pudo crear interfaz TUN")?;
        let name = dev.tun_name().context("No se pudo obtener nombre de TUN")?;

        // Ajustar IPs y MTU (macOS requiere ifconfig)
        run(
            "ifconfig",
            &[&name, &ip.host.to_string(), &ip.peer.to_string(), "up"],
        )?;
        run("ifconfig", &[&name, "mtu", &ip.mtu.to_string()])?;

        info!("TUN creada: {} ({} <-> {})", name, ip.host, ip.peer);
        Ok(Self { name, _dev: dev })
    }

    pub fn down(&mut self) -> Result<()> {
        run("ifconfig", &[&self.name, "down"])?;
        info!("TUN {} abajo", self.name);
        Ok(())
    }
}

fn run(cmd: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .context(cmd.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{} {:?} falló: {}", cmd, args, stderr);
    }
    Ok(())
}
