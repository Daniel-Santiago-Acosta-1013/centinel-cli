use anyhow::{Context, Result};
use log::info;
use std::collections::HashMap;
use std::process::Command;

pub struct DnsOverride {
    device: String,
    service: String,
    prev: Vec<String>,
}

impl DnsOverride {
    pub fn new(device: String) -> Result<Self> {
        let service = device_to_service(&device)?;
        Ok(Self {
            device,
            service,
            prev: Vec::new(),
        })
    }

    pub fn apply_local(&mut self) -> Result<()> {
        self.prev = current_dns(&self.service)?;
        run(
            "networksetup",
            &["-setdnsservers", &self.service, "127.0.0.1"],
        )?;
        info!(
            "DNS del sistema dirigido a 127.0.0.1 (servicio: {}, dispositivo: {})",
            self.service, self.device
        );
        Ok(())
    }

    pub fn restore(&mut self) -> Result<()> {
        if self.prev.is_empty() {
            run("networksetup", &["-setdnsservers", &self.service, "Empty"])?;
        } else {
            let mut args = vec!["-setdnsservers", &self.service];
            let servers: Vec<String> = self.prev.clone();
            for s in &servers {
                args.push(s);
            }
            run("networksetup", &args)?;
        }
        info!(
            "DNS restaurado en servicio {} ({}).",
            self.service, self.device
        );
        Ok(())
    }
}

fn current_dns(service: &str) -> Result<Vec<String>> {
    let out = Command::new("networksetup")
        .args(["-getdnsservers", service])
        .output()
        .context("networksetup -getdnsservers")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!(
            "networksetup -getdnsservers falló para servicio '{}': {}",
            service,
            stderr
        );
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("There aren't any DNS Servers") {
        return Ok(Vec::new());
    }
    let servers: Vec<String> = stdout.lines().map(|l| l.trim().to_string()).collect();
    Ok(servers)
}

fn device_to_service(device: &str) -> Result<String> {
    let out = Command::new("networksetup")
        .arg("-listallhardwareports")
        .output()
        .context("networksetup -listallhardwareports")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("networksetup -listallhardwareports falló: {}", stderr);
    }
    let map = parse_hwports(&String::from_utf8_lossy(&out.stdout));
    map.get(device)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No se encontró un servicio de red para el dispositivo {device}. Usa networksetup -listallhardwareports para verificar."))
}

fn parse_hwports(out: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current_service: Option<String> = None;
    for line in out.lines() {
        let line = line.trim();
        if line.starts_with("Hardware Port:") {
            current_service = line.splitn(2, ':').nth(1).map(|s| s.trim().to_string());
        } else if line.starts_with("Device:") {
            if let (Some(svc), Some(dev)) = (
                current_service.clone(),
                line.splitn(2, ':').nth(1).map(|s| s.trim().to_string()),
            ) {
                map.insert(dev, svc);
            }
        }
    }
    map
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

#[cfg(test)]
mod tests {
    use super::parse_hwports;
    use std::collections::HashMap;

    #[test]
    fn parse_devices() {
        let sample = r#"
Hardware Port: Wi-Fi
Device: en0
Ethernet Address: aa:bb:cc:dd:ee:ff

Hardware Port: Thunderbolt Ethernet
Device: en5
Ethernet Address: ff:ee:dd:cc:bb:aa
        "#;
        let map = parse_hwports(sample);
        let mut expected = HashMap::new();
        expected.insert("en0".to_string(), "Wi-Fi".to_string());
        expected.insert("en5".to_string(), "Thunderbolt Ethernet".to_string());
        assert_eq!(map, expected);
    }
}
