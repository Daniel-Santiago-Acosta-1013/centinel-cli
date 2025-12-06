use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

const DEFAULT_BLOCKLIST: &str = include_str!("../../data/blocklist.txt");

pub struct Blocklist {
    path: Option<PathBuf>,
}

impl Blocklist {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<Vec<String>> {
        let mut entries: HashSet<String> = HashSet::new();

        if let Some(path) = &self.path {
            if path.exists() {
                let content = fs::read_to_string(path).with_context(|| {
                    format!("No se pudo leer la blocklist en {}", path.display())
                })?;
                for line in content.lines() {
                    Self::push_line(&mut entries, line);
                }
            }
        }

        for line in DEFAULT_BLOCKLIST.lines() {
            Self::push_line(&mut entries, line);
        }

        let mut out: Vec<String> = entries.into_iter().collect();
        out.sort();
        Ok(out)
    }

    fn push_line(entries: &mut HashSet<String>, line: &str) {
        let clean = line.trim();
        if clean.is_empty() || clean.starts_with('#') {
            return;
        }
        let normalized = clean.trim().to_lowercase();
        entries.insert(normalized);
    }
}
