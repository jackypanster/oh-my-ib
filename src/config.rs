//! Runtime configuration. Precedence: CLI flag > config.toml > built-in default.
//! The config file lives at ~/.config/oh-my-ib/config.toml (outside this public repo).

use std::path::PathBuf;

use serde::Deserialize;

use crate::cli::GlobalOpts;
use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdType {
    Live,
    Delayed,
    Frozen,
}

impl MdType {
    pub fn parse(s: &str) -> Result<MdType, AppError> {
        match s.to_ascii_lowercase().as_str() {
            "live" => Ok(MdType::Live),
            "delayed" => Ok(MdType::Delayed),
            "frozen" => Ok(MdType::Frozen),
            other => Err(AppError::config(
                format!("invalid md-type: {other}"),
                "expected one of: live|delayed|frozen",
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub client_id: i32,
    pub account: Option<String>,
    pub md_type: MdType,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 4002, // paper
            client_id: 100,
            account: None,
            md_type: MdType::Delayed,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    host: Option<String>,
    port: Option<u16>,
    client_id: Option<i32>,
    account: Option<String>,
    md_type: Option<String>,
}

impl Config {
    pub fn config_path() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(".config/oh-my-ib/config.toml"))
    }

    /// Load defaults, then overlay the config file if present.
    pub fn load() -> Result<Config, AppError> {
        let mut cfg = Config::default();
        if let Some(path) = Self::config_path() {
            if path.exists() {
                let text = std::fs::read_to_string(&path).map_err(|e| {
                    AppError::config(format!("read config: {e}"), path.display().to_string())
                })?;
                let file: FileConfig = toml::from_str(&text).map_err(|e| {
                    AppError::config(format!("parse config: {e}"), path.display().to_string())
                })?;
                cfg.apply_file(file)?;
            }
        }
        Ok(cfg)
    }

    fn apply_file(&mut self, file: FileConfig) -> Result<(), AppError> {
        if let Some(host) = file.host {
            self.host = host;
        }
        if let Some(port) = file.port {
            self.port = port;
        }
        if let Some(client_id) = file.client_id {
            self.client_id = client_id;
        }
        if file.account.is_some() {
            self.account = file.account;
        }
        if let Some(md) = file.md_type {
            self.md_type = MdType::parse(&md)?;
        }
        Ok(())
    }

    /// Overlay CLI global flags (highest precedence).
    pub fn merge_flags(mut self, g: &GlobalOpts) -> Result<Config, AppError> {
        if let Some(host) = &g.host {
            self.host = host.clone();
        }
        if g.live {
            self.port = 4001; // live
        }
        if let Some(port) = g.port {
            self.port = port; // explicit --port wins over --live
        }
        if let Some(client_id) = g.client_id {
            self.client_id = client_id;
        }
        if g.account.is_some() {
            self.account = g.account.clone();
        }
        Ok(self)
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_paper() {
        let c = Config::default();
        assert_eq!(c.host, "127.0.0.1");
        assert_eq!(c.port, 4002);
        assert_eq!(c.client_id, 100);
        assert_eq!(c.md_type, MdType::Delayed);
    }

    #[test]
    fn live_flag_selects_live_port() {
        let g = GlobalOpts {
            live: true,
            ..Default::default()
        };
        let c = Config::default().merge_flags(&g).unwrap();
        assert_eq!(c.port, 4001);
    }

    #[test]
    fn explicit_port_wins_over_live() {
        let g = GlobalOpts {
            live: true,
            port: Some(4002),
            ..Default::default()
        };
        let c = Config::default().merge_flags(&g).unwrap();
        assert_eq!(c.port, 4002);
    }

    #[test]
    fn flag_overrides_host() {
        let g = GlobalOpts {
            host: Some("10.0.0.5".to_string()),
            ..Default::default()
        };
        let c = Config::default().merge_flags(&g).unwrap();
        assert_eq!(c.host, "10.0.0.5");
        assert_eq!(c.address(), "10.0.0.5:4002");
    }

    #[test]
    fn md_type_parse() {
        assert_eq!(MdType::parse("DELAYED").unwrap(), MdType::Delayed);
        assert_eq!(MdType::parse("live").unwrap(), MdType::Live);
        assert!(MdType::parse("bogus").is_err());
    }
}
