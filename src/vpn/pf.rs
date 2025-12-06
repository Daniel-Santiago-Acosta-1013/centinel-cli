use anyhow::{Context, Result};
use log::info;
use std::process::Command;
use std::{fs, path::PathBuf};

pub struct PfState {
    iface_out: String,
    cidr: String,
    anchor_file: PathBuf,
    enabled_before: bool,
}

impl PfState {
    pub fn new(iface_out: String, cidr: String) -> Self {
        Self {
            iface_out,
            cidr,
            anchor_file: PathBuf::from("/tmp/sentinel_anchor.pf"),
            enabled_before: false,
        }
    }

    pub fn enable(&mut self, tun_name: &str) -> Result<()> {
        let info = Command::new("pfctl").args(["-s", "info"]).output()?;
        let info_str = String::from_utf8_lossy(&info.stdout);
        self.enabled_before = info_str.contains("Status: Enabled");

        // Generar reglas en un ancla propia
        let rules = format!(
            "nat on {iface} from {cidr} to any -> ({iface})\n\
             pass quick on {tun} inet proto {{ tcp udp icmp }} from {cidr} to any keep state\n\
             pass quick on {iface} inet proto {{ tcp udp icmp }} from any to {cidr} keep state\n",
            iface = self.iface_out,
            cidr = self.cidr,
            tun = tun_name
        );
        fs::write(&self.anchor_file, rules)?;
        run("pfctl", &["-a", "sentinel", "-F", "all"])?;
        run(
            "pfctl",
            &[
                "-a",
                "sentinel",
                "-f",
                self.anchor_file.to_string_lossy().as_ref(),
            ],
        )?;

        if !self.enabled_before {
            run("pfctl", &["-E"])?;
        }

        info!(
            "PF configurado con NAT en {} para {}",
            self.iface_out, self.cidr
        );
        Ok(())
    }

    pub fn disable(&mut self) -> Result<()> {
        let _ = run_ignore("pfctl", &["-a", "sentinel", "-F", "all"]);
        if !self.enabled_before {
            let _ = run_ignore("pfctl", &["-d"]);
        }
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
