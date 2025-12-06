use anyhow::{Context, Result};
use log::info;
use std::process::Command;

pub struct IpForwardGuard {
    prev: i32,
}

impl IpForwardGuard {
    pub fn enable() -> Result<Self> {
        let prev = read_forward()?;
        if prev != 1 {
            write_forward(1)?;
            info!("IP forwarding habilitado (previo={prev})");
        } else {
            info!("IP forwarding ya estaba habilitado");
        }
        Ok(Self { prev })
    }

    pub fn restore(self) -> Result<()> {
        if self.prev != 1 {
            write_forward(self.prev)?;
            info!("IP forwarding restaurado a {}", self.prev);
        }
        Ok(())
    }
}

fn read_forward() -> Result<i32> {
    let out = Command::new("sysctl")
        .args(["-n", "net.inet.ip.forwarding"])
        .output()
        .context("sysctl -n net.inet.ip.forwarding")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("No se pudo leer ip.forwarding: {stderr}");
    }
    let val = String::from_utf8_lossy(&out.stdout).trim().parse::<i32>()?;
    Ok(val)
}

fn write_forward(val: i32) -> Result<()> {
    let out = Command::new("sysctl")
        .args(["-w", &format!("net.inet.ip.forwarding={val}")])
        .output()
        .context("sysctl -w net.inet.ip.forwarding")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("No se pudo establecer ip.forwarding={val}: {stderr}");
    }
    Ok(())
}
