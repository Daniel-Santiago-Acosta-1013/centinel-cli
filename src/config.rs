use directories::ProjectDirs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Config {
    pub blocklist_path: Option<PathBuf>,
    pub hosts_path: PathBuf,
    pub backup_path: PathBuf,
    pub vpn: VpnConfig,
}

#[derive(Clone)]
pub struct VpnConfig {
    pub wg_profile: String,
    pub openvpn_config: PathBuf,
    pub profile_name: String,
}

impl Default for Config {
    fn default() -> Self {
        let project_dirs = ProjectDirs::from("com", "sentinel", "sentinel")
            .expect("no se pudo resolver el directorio de configuración");
        let config_dir = project_dirs.config_dir().to_path_buf();

        let hosts_path = default_hosts_path();
        let backup_path = config_dir.join("hosts.bak");
        let blocklist_path = Some(config_dir.join("blocklist.txt"));

        let vpn = VpnConfig {
            wg_profile: "sentinel".to_string(),
            openvpn_config: config_dir.join("vpn").join("sentinel.ovpn"),
            profile_name: "SentinelVPN".to_string(),
        };

        Self {
            blocklist_path,
            hosts_path,
            backup_path,
            vpn,
        }
    }
}

fn default_hosts_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        Path::new(r"C:\Windows\System32\drivers\etc\hosts").to_path_buf()
    } else {
        Path::new("/etc/hosts").to_path_buf()
    }
}
