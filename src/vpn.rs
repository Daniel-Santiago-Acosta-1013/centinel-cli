use crate::config::VpnConfig;
use anyhow::{Context, Result, bail};
use log::{info, warn};
use std::process::Command;
use which::which;

#[derive(Clone, Copy, Debug)]
pub enum VpnState {
    On,
    Off,
    Unknown,
}

impl VpnState {
    pub fn as_str(&self) -> &'static str {
        match self {
            VpnState::On => "encendida",
            VpnState::Off => "apagada",
            VpnState::Unknown => "desconocido",
        }
    }
}

pub struct VpnController {
    config: VpnConfig,
    state: VpnState,
}

impl VpnController {
    pub fn new(config: VpnConfig) -> Self {
        Self {
            config,
            state: VpnState::Unknown,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        info!("Intentando encender la VPN");
        let started =
            self.try_wg_quick("up")? || self.try_platform_start()? || self.try_openvpn_start()?;

        if started {
            self.state = VpnState::On;
            Ok(())
        } else {
            bail!("No se encontraron herramientas compatibles (wg-quick, openvpn, scutil/nmcli)");
        }
    }

    pub fn stop(&mut self) -> Result<()> {
        info!("Intentando apagar la VPN");
        let stopped =
            self.try_wg_quick("down")? || self.try_platform_stop()? || self.try_openvpn_stop()?;

        if stopped {
            self.state = VpnState::Off;
            Ok(())
        } else {
            bail!("No se pudieron ejecutar comandos para detener la VPN");
        }
    }

    pub fn status(&self) -> VpnState {
        self.state
    }

    fn try_wg_quick(&self, action: &str) -> Result<bool> {
        if let Ok(cmd) = which("wg-quick") {
            let profile = &self.config.wg_profile;
            let success = run_command(cmd.to_string_lossy().as_ref(), &[action, profile])
                .with_context(|| format!("wg-quick {action} {profile}"))?;
            return Ok(success);
        }
        Ok(false)
    }

    fn try_openvpn_start(&self) -> Result<bool> {
        if let Ok(cmd) = which("openvpn") {
            let config = &self.config.openvpn_config;
            if config.exists() {
                let success = run_command(
                    cmd.to_string_lossy().as_ref(),
                    &["--config", config.to_string_lossy().as_ref(), "--daemon"],
                )
                .with_context(|| "openvpn --config ...")?;
                return Ok(success);
            } else {
                warn!(
                    "No se encontró el archivo de configuración de OpenVPN en {}",
                    config.display()
                );
            }
        }
        Ok(false)
    }

    fn try_openvpn_stop(&self) -> Result<bool> {
        // Best-effort: enviar señal SIGTERM al proceso openvpn asociado al perfil.
        if let Ok(cmd) = which("pkill") {
            let success = run_command(cmd.to_string_lossy().as_ref(), &["-f", "openvpn.*sentinel"])
                .with_context(|| "pkill openvpn sentinel")?;
            return Ok(success);
        }
        Ok(false)
    }

    fn try_platform_start(&self) -> Result<bool> {
        #[cfg(target_os = "macos")]
        {
            if let Ok(cmd) = which("scutil") {
                let success = run_command(
                    cmd.to_string_lossy().as_ref(),
                    &["--nc", "start", &self.config.profile_name],
                )
                .with_context(|| "scutil --nc start")?;
                return Ok(success);
            }
            if let Ok(cmd) = which("networksetup") {
                let success = run_command(
                    cmd.to_string_lossy().as_ref(),
                    &["-connectpppoeservice", &self.config.profile_name],
                )
                .with_context(|| "networksetup -connectpppoeservice")?;
                return Ok(success);
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(cmd) = which("nmcli") {
                let success = run_command(
                    cmd.to_string_lossy().as_ref(),
                    &["connection", "up", &self.config.profile_name],
                )
                .with_context(|| "nmcli connection up")?;
                return Ok(success);
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(cmd) = which("rasdial") {
                let success =
                    run_command(cmd.to_string_lossy().as_ref(), &[&self.config.profile_name])
                        .with_context(|| "rasdial profile")?;
                return Ok(success);
            }
        }

        Ok(false)
    }

    fn try_platform_stop(&self) -> Result<bool> {
        #[cfg(target_os = "macos")]
        {
            if let Ok(cmd) = which("scutil") {
                let success = run_command(
                    cmd.to_string_lossy().as_ref(),
                    &["--nc", "stop", &self.config.profile_name],
                )
                .with_context(|| "scutil --nc stop")?;
                return Ok(success);
            }
            if let Ok(cmd) = which("networksetup") {
                let success = run_command(
                    cmd.to_string_lossy().as_ref(),
                    &["-disconnectpppoeservice", &self.config.profile_name],
                )
                .with_context(|| "networksetup -disconnectpppoeservice")?;
                return Ok(success);
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(cmd) = which("nmcli") {
                let success = run_command(
                    cmd.to_string_lossy().as_ref(),
                    &["connection", "down", &self.config.profile_name],
                )
                .with_context(|| "nmcli connection down")?;
                return Ok(success);
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(cmd) = which("rasdial") {
                let success = run_command(
                    cmd.to_string_lossy().as_ref(),
                    &[&self.config.profile_name, "/disconnect"],
                )
                .with_context(|| "rasdial profile /disconnect")?;
                return Ok(success);
            }
        }

        Ok(false)
    }
}

fn run_command(cmd: &str, args: &[&str]) -> Result<bool> {
    let output = Command::new(cmd).args(args).output()?;
    if output.status.success() {
        info!("Comando `{cmd}` ejecutado correctamente");
        Ok(true)
    } else {
        warn!(
            "Comando `{cmd}` falló con código {:?}",
            output.status.code()
        );
        Ok(false)
    }
}
