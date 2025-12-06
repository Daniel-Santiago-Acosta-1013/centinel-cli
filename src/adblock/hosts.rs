use anyhow::{Context, Result};
use log::{info, warn};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

const SENTINEL_HEADER: &str = "# >>> sentinel adblock START";
const SENTINEL_FOOTER: &str = "# <<< sentinel adblock END";

pub struct HostsManager {
    hosts_path: PathBuf,
    backup_path: PathBuf,
}

impl HostsManager {
    pub fn new(hosts_path: PathBuf, backup_path: PathBuf) -> Self {
        Self {
            hosts_path,
            backup_path,
        }
    }

    pub fn apply(&self, blocklist: &[String]) -> Result<()> {
        self.ensure_backup()?;
        let mut content = fs::read_to_string(&self.hosts_path)
            .unwrap_or_else(|_| String::from("# Archivo hosts generado por sentinel\n"));

        content = strip_sentinel_section(&content);

        let mut section = String::new();
        section.push_str(SENTINEL_HEADER);
        section.push('\n');
        section.push_str("# Entradas añadidas por sentinel. Remuévelas con 'sentinel'.\n");
        for domain in blocklist {
            section.push_str("0.0.0.0 ");
            section.push_str(domain);
            section.push('\n');
        }
        section.push_str(SENTINEL_FOOTER);
        section.push('\n');

        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&section);

        fs::write(&self.hosts_path, content).with_context(|| {
            format!(
                "No se pudo escribir en {}. Ejecuta con privilegios de administrador.",
                self.hosts_path.display()
            )
        })?;

        info!(
            "Se añadieron {} dominios bloqueados en {}",
            blocklist.len(),
            self.hosts_path.display()
        );
        Ok(())
    }

    pub fn remove(&self) -> Result<()> {
        if !self.hosts_path.exists() {
            warn!(
                "El archivo hosts no existe en {}",
                self.hosts_path.display()
            );
            return Ok(());
        }

        let content = fs::read_to_string(&self.hosts_path)?;
        let stripped = strip_sentinel_section(&content);

        // Si hay respaldo preferimos restaurar el original.
        if self.backup_path.exists() {
            fs::copy(&self.backup_path, &self.hosts_path).with_context(|| {
                format!(
                    "No se pudo restaurar el respaldo desde {}",
                    self.backup_path.display()
                )
            })?;
            info!("Se restauró el archivo hosts desde el respaldo.");
            return Ok(());
        }

        fs::write(&self.hosts_path, stripped)?;
        info!("Se removieron las entradas de sentinel del archivo hosts.");
        Ok(())
    }

    fn ensure_backup(&self) -> Result<()> {
        if self.backup_path.exists() {
            return Ok(());
        }
        if let Some(dir) = self.backup_path.parent() {
            fs::create_dir_all(dir)
                .with_context(|| format!("No se pudo crear {}", dir.display()))?;
        }
        if self.hosts_path.exists() {
            fs::copy(&self.hosts_path, &self.backup_path)
                .with_context(|| "No se pudo crear el respaldo de hosts")?;
            info!("Respaldo de hosts creado en {}", self.backup_path.display());
        } else {
            // Si no existe, creamos un archivo hosts mínimo.
            let mut file = fs::File::create(&self.hosts_path)
                .with_context(|| "No se pudo crear el archivo hosts")?;
            file.write_all(b"127.0.0.1 localhost\n")?;
            info!(
                "Se generó un archivo hosts vacío en {}",
                self.hosts_path.display()
            );
        }
        Ok(())
    }
}

fn strip_sentinel_section(content: &str) -> String {
    let mut output = String::new();
    let mut skip = false;
    for line in content.lines() {
        if line.trim() == SENTINEL_HEADER {
            skip = true;
            continue;
        }
        if line.trim() == SENTINEL_FOOTER {
            skip = false;
            continue;
        }
        if !skip {
            output.push_str(line);
            output.push('\n');
        }
    }
    if output.ends_with('\n') {
        output
    } else {
        let mut with_newline = output;
        with_newline.push('\n');
        with_newline
    }
}
