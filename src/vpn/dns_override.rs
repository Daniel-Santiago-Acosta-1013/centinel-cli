use anyhow::{Context, Result};
use log::info;
use std::process::Command;

pub struct DnsOverride {
    iface: String,
    prev: Vec<String>,
}

impl DnsOverride {
    pub fn new(iface: String) -> Self {
        Self {
            iface,
            prev: Vec::new(),
        }
    }

    pub fn apply_local(&mut self) -> Result<()> {
        self.prev = current_dns(&self.iface)?;
        run(
            "networksetup",
            &["-setdnsservers", &self.iface, "127.0.0.1"],
        )?;
        info!(
            "DNS del sistema dirigido a 127.0.0.1 para la interfaz {}",
            self.iface
        );
        Ok(())
    }

    pub fn restore(&mut self) -> Result<()> {
        if self.prev.is_empty() {
            run("networksetup", &["-setdnsservers", &self.iface, "Empty"])?;
        } else {
            let mut args = vec!["-setdnsservers", &self.iface];
            let servers: Vec<String> = self.prev.clone();
            for s in &servers {
                args.push(s);
            }
            run("networksetup", &args)?;
        }
        info!("DNS restaurado en {}", self.iface);
        Ok(())
    }
}

fn current_dns(iface: &str) -> Result<Vec<String>> {
    let out = Command::new("networksetup")
        .args(["-getdnsservers", iface])
        .output()
        .context("networksetup -getdnsservers")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("networksetup -getdnsservers falló: {}", stderr);
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("There aren't any DNS Servers") {
        return Ok(Vec::new());
    }
    let servers: Vec<String> = stdout.lines().map(|l| l.trim().to_string()).collect();
    Ok(servers)
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
