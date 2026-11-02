use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    #[serde(default = "default_title")]
    pub title: String,
    #[serde(default)]
    pub announcement: String,
}

impl Default for Site {
    fn default() -> Self {
        Self {
            title: default_title(),
            announcement: String::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    #[serde(default)]
    pub id: String,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type", default)]
    pub node_type: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub disabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_http_listen")]
    pub http_listen: String,
    #[serde(default)]
    pub site: Site,
    #[serde(default)]
    pub servers: Vec<Server>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let data = std::fs::read_to_string(path)
            .with_context(|| format!("read config {}", path.display()))?;
        let mut cfg: Config = serde_json::from_str(&data).context("parse config json")?;
        cfg.normalize();
        cfg.validate()?;
        Ok(cfg)
    }

    fn normalize(&mut self) {
        if self.listen.is_empty() {
            self.listen = default_listen();
        }
        if self.http_listen.is_empty() {
            self.http_listen = default_http_listen();
        }
        if self.site.title.trim().is_empty() {
            self.site.title = default_title();
        }
        for server in &mut self.servers {
            if server.id.is_empty() {
                server.id = server.username.clone();
            }
            if server.name.is_empty() {
                server.name = server.username.clone();
            }
            if server.tags.is_empty() {
                server.tags = Vec::new();
            }
        }
    }

    fn validate(&self) -> Result<()> {
        let mut usernames = HashSet::new();
        let mut ids = HashSet::new();

        for (i, server) in self.servers.iter().enumerate() {
            if server.username.trim().is_empty() {
                bail!("servers[{i}]: username is required");
            }
            if server.password.is_empty() {
                bail!("servers[{i}] ({}): password is required", server.username);
            }
            if server.username.contains(':') {
                bail!(
                    "servers[{i}] ({}): username must not contain ':'",
                    server.username
                );
            }
            if !usernames.insert(server.username.clone()) {
                bail!("servers[{i}] ({}): duplicate username", server.username);
            }
            if server.id.trim().is_empty() {
                bail!("servers[{i}] ({}): id is required", server.username);
            }
            if !ids.insert(server.id.clone()) {
                bail!(
                    "servers[{i}] ({}): duplicate id {}",
                    server.username,
                    server.id
                );
            }
        }

        Ok(())
    }
}

fn default_title() -> String {
    "ServerStatus-Plus".to_string()
}

fn default_listen() -> String {
    ":35601".to_string()
}

fn default_http_listen() -> String {
    ":8080".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_original_toyo_config() {
        let json = include_str!("../tests/fixtures/original-config.json");
        let mut cfg: Config = serde_json::from_str(json).expect("parse original config");
        cfg.normalize();
        cfg.validate().expect("validate original config");

        assert_eq!(cfg.listen, ":35601");
        assert_eq!(cfg.http_listen, ":8080");
        assert_eq!(cfg.site.title, "ServerStatus-Plus");
        assert_eq!(cfg.site.announcement, "");
        assert_eq!(cfg.servers[0].id, "s01");
        assert_eq!(cfg.servers[0].name, "Mainserver 1");
        assert!(cfg.servers[1].disabled);
    }
}
