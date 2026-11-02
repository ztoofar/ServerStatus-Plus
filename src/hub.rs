use crate::config::Server;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub host: String,
    pub location: String,
    pub connected: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub connected_via: String,
    pub last_seen: i64,
    pub uptime: i64,
    pub load: f64,
    pub network_rx: i64,
    pub network_tx: i64,
    pub network_in: i64,
    pub network_out: i64,
    pub memory_total: i64,
    pub memory_used: i64,
    pub swap_total: i64,
    pub swap_used: i64,
    pub hdd_total: i64,
    pub hdd_used: i64,
    pub cpu: f64,
    pub online4: bool,
    pub online6: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub custom: String,
    #[serde(skip)]
    pub username: String,
    #[serde(skip)]
    pub disabled: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Update {
    pub uptime: Option<i64>,
    pub load: Option<f64>,
    pub network_rx: Option<i64>,
    pub network_tx: Option<i64>,
    pub network_in: Option<i64>,
    pub network_out: Option<i64>,
    pub memory_total: Option<i64>,
    pub memory_used: Option<i64>,
    pub swap_total: Option<i64>,
    pub swap_used: Option<i64>,
    pub hdd_total: Option<i64>,
    pub hdd_used: Option<i64>,
    pub cpu: Option<f64>,
    pub online4: Option<bool>,
    pub online6: Option<bool>,
    pub custom: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConnectError {
    AlreadyConnected,
    Unknown,
}

pub struct Hub {
    pub(crate) nodes: Vec<Node>,
    by_id: HashMap<String, usize>,
    by_user: HashMap<String, (usize, String)>,
}

impl Hub {
    pub fn new(servers: &[Server]) -> Self {
        let mut nodes = Vec::with_capacity(servers.len());
        let mut by_id = HashMap::with_capacity(servers.len());
        let mut by_user = HashMap::with_capacity(servers.len());

        for server in servers {
            let index = nodes.len();
            let node = Node {
                id: server.id.clone(),
                name: server.name.clone(),
                node_type: server.node_type.clone(),
                host: server.host.clone(),
                location: server.location.clone(),
                connected: false,
                connected_via: String::new(),
                last_seen: 0,
                uptime: 0,
                load: 0.0,
                network_rx: 0,
                network_tx: 0,
                network_in: 0,
                network_out: 0,
                memory_total: 0,
                memory_used: 0,
                swap_total: 0,
                swap_used: 0,
                hdd_total: 0,
                hdd_used: 0,
                cpu: 0.0,
                online4: false,
                online6: false,
                custom: String::new(),
                username: server.username.clone(),
                disabled: server.disabled,
            };
            by_id.insert(node.id.clone(), index);
            by_user.insert(server.username.clone(), (index, server.password.clone()));
            nodes.push(node);
        }

        Self {
            nodes,
            by_id,
            by_user,
        }
    }

    /// Returns (node index, node id, username) on success.
    pub fn authenticate(&self, username: &str, password: &str) -> Option<(usize, String, String)> {
        let &(index, ref stored) = self.by_user.get(username)?;
        let node = self.nodes.get(index)?;
        if node.disabled {
            return None;
        }
        if password.as_bytes().ct_eq(stored.as_bytes()).unwrap_u8() == 1 {
            Some((index, node.id.clone(), node.username.clone()))
        } else {
            None
        }
    }

    pub fn connect(&mut self, index: usize, via: &str) -> Result<(), ConnectError> {
        let node = self.nodes.get_mut(index).ok_or(ConnectError::Unknown)?;
        if node.disabled {
            return Err(ConnectError::Unknown);
        }
        if node.connected {
            return Err(ConnectError::AlreadyConnected);
        }
        node.connected = true;
        node.connected_via = via.to_string();
        node.last_seen = now_unix();
        match via {
            "v4" => node.online4 = true,
            "v6" => node.online6 = true,
            _ => {}
        }
        Ok(())
    }

    pub fn disconnect(&mut self, index: usize) {
        if let Some(node) = self.nodes.get_mut(index) {
            node.connected = false;
            node.connected_via.clear();
            node.online4 = false;
            node.online6 = false;
            node.last_seen = now_unix();
        }
    }

    pub fn update(&mut self, index: usize, update: &Update) {
        let Some(node) = self.nodes.get_mut(index) else {
            return;
        };
        if !node.connected {
            return;
        }

        if let Some(value) = update.uptime {
            node.uptime = value;
        }
        if let Some(value) = update.load {
            node.load = value;
        }
        if let Some(value) = update.network_rx {
            node.network_rx = value;
        }
        if let Some(value) = update.network_tx {
            node.network_tx = value;
        }
        if let Some(value) = update.network_in {
            node.network_in = value;
        }
        if let Some(value) = update.network_out {
            node.network_out = value;
        }
        if let Some(value) = update.memory_total {
            node.memory_total = value;
        }
        if let Some(value) = update.memory_used {
            node.memory_used = value;
        }
        if let Some(value) = update.swap_total {
            node.swap_total = value;
        }
        if let Some(value) = update.swap_used {
            node.swap_used = value;
        }
        if let Some(value) = update.hdd_total {
            node.hdd_total = value;
        }
        if let Some(value) = update.hdd_used {
            node.hdd_used = value;
        }
        if let Some(value) = update.cpu {
            node.cpu = value;
        }
        if let Some(value) = &update.custom {
            node.custom = value.clone();
        }

        // The connection family is authoritative for its own online flag. The
        // client reports the *other* family, matching the original protocol.
        if let Some(value) = update.online4 {
            if node.connected_via == "v6" {
                node.online4 = value;
            }
        }
        if let Some(value) = update.online6 {
            if node.connected_via == "v4" {
                node.online6 = value;
            }
        }

        node.last_seen = now_unix();
    }

    pub fn snapshot(&self) -> Vec<Node> {
        self.nodes
            .iter()
            .filter(|node| !node.disabled)
            .cloned()
            .collect()
    }

    pub fn find(&self, id: &str) -> Option<Node> {
        self.by_id
            .get(id)
            .and_then(|&index| self.nodes.get(index))
            .filter(|node| !node.disabled)
            .cloned()
    }
}

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
