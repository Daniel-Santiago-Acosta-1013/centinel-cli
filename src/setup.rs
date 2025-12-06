use crate::config::Config;
use crate::vpn::VpnTools;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

const BASE_BLOCKLIST: &str = include_str!("../data/blocklist.txt");

pub struct SetupReport {
    pub config_dir: PathBuf,
    pub blocklist_path: Option<PathBuf>,
    pub vpn_config_path: PathBuf,
    pub created_blocklist: bool,
    pub created_vpn_dir: bool,
    pub tools: VpnTools,
}

pub fn ensure(config: &mut Config) -> Result<SetupReport> {
    let mut created_blocklist = false;
    let mut created_vpn_dir = false;

    if !config.config_dir.exists() {
        fs::create_dir_all(&config.config_dir)?;
    }

    // Blocklist personalizada (copiamos base si no existe).
    if let Some(path) = &config.blocklist_path {
        if !path.exists() {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            fs::write(path, BASE_BLOCKLIST)?;
            created_blocklist = true;
        }
    }

    // Carpeta para perfiles de VPN.
    let vpn_dir = config.config_dir.join("vpn");
    if !vpn_dir.exists() {
        fs::create_dir_all(&vpn_dir)?;
        created_vpn_dir = true;
    }

    // Crear config embebida mínima si no existe.
    if !config.vpn.embedded_config.exists() {
        if let Some(dir) = config.vpn.embedded_config.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(
            &config.vpn.embedded_config,
            "# sentinel vpn embebida\n# añade aquí endpoints si los tienes\n",
        )?;
    }

    let tools = VpnTools::detect();

    Ok(SetupReport {
        config_dir: config.config_dir.clone(),
        blocklist_path: config.blocklist_path.clone(),
        vpn_config_path: config.vpn.embedded_config.clone(),
        created_blocklist,
        created_vpn_dir,
        tools,
    })
}
