use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::Mutex;
use tokio::time::{interval, Duration};

const DISCOVERY_PORT: u16 = 9091;
const ANNOUNCE_INTERVAL_SECS: u64 = 10;
const STALE_TIMEOUT_SECS: i64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshNode {
    pub node_id: String,
    pub display_name: String,
    pub status: String,
    pub last_seen: String,
    pub address: String,
    pub capabilities: Vec<String>,
    pub mesh_port: u16,
}

static DISCOVERED_NODES: Mutex<Option<HashMap<String, MeshNode>>> = Mutex::new(None);

fn nodes_map() -> &'static Mutex<Option<HashMap<String, MeshNode>>> {
    &DISCOVERED_NODES
}

fn ensure_init() {
    let mut guard = nodes_map().lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
}

/// Get the current list of discovered nodes
pub fn get_discovered_nodes() -> Vec<MeshNode> {
    ensure_init();
    let guard = nodes_map().lock().unwrap();
    match guard.as_ref() {
        Some(map) => map.values().cloned().collect(),
        None => Vec::new(),
    }
}

/// Parse an announcement message: AGENTOS|node_id|hostname|mesh_port
fn parse_announce(msg: &str, sender_ip: &str) -> Option<MeshNode> {
    let parts: Vec<&str> = msg.split('|').collect();
    if parts.len() == 4 && parts[0] == "AGENTOS" {
        let node_id = parts[1].to_string();
        let hostname = parts[2].to_string();
        let mesh_port: u16 = parts[3].parse().ok()?;
        Some(MeshNode {
            node_id,
            display_name: hostname,
            status: "online".to_string(),
            last_seen: chrono::Utc::now().to_rfc3339(),
            address: format!("{}:{}", sender_ip, mesh_port),
            capabilities: vec!["chat".to_string(), "screen".to_string(), "cli".to_string()],
            mesh_port,
        })
    } else {
        None
    }
}

fn add_node(node: MeshNode) {
    ensure_init();
    let mut guard = nodes_map().lock().unwrap();
    if let Some(map) = guard.as_mut() {
        let id = node.node_id.clone();
        map.insert(id, node);
    }
}

fn remove_stale_nodes() {
    ensure_init();
    let mut guard = nodes_map().lock().unwrap();
    if let Some(map) = guard.as_mut() {
        let now = chrono::Utc::now();
        map.retain(|_id, node| {
            if let Ok(last) = chrono::DateTime::parse_from_rfc3339(&node.last_seen) {
                let age = now.signed_duration_since(last.with_timezone(&chrono::Utc));
                age.num_seconds() < STALE_TIMEOUT_SECS
            } else {
                false
            }
        });
    }
}

/// Start UDP broadcast discovery (runs in background)
///
/// Broadcasts our presence every 10 seconds on port 9091 and listens for
/// other AgentOS instances doing the same. Stale nodes (not seen for 30s)
/// are automatically removed.
pub async fn start_discovery(node_name: &str, mesh_port: u16) -> Result<(), String> {
    ensure_init();

    let node_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    tracing::info!(
        "Mesh discovery started: node_id={}, name={}, mesh_port={}",
        node_id,
        node_name,
        mesh_port
    );

    // Register ourselves
    let self_node = MeshNode {
        node_id: node_id.clone(),
        display_name: node_name.to_string(),
        status: "online".to_string(),
        last_seen: chrono::Utc::now().to_rfc3339(),
        address: format!("127.0.0.1:{}", mesh_port),
        capabilities: vec!["chat".to_string(), "screen".to_string(), "cli".to_string()],
        mesh_port,
    };
    add_node(self_node.clone());

    let announce_msg = format!("AGENTOS|{}|{}|{}", node_id, node_name, mesh_port);
    let our_node_id = node_id.clone();

    tokio::spawn(async move {
        // Try to bind the UDP socket for discovery
        let socket = match UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT)) {
            Ok(s) => {
                s.set_broadcast(true).ok();
                s.set_nonblocking(true).ok();
                tracing::info!("Mesh discovery socket bound on port {}", DISCOVERY_PORT);
                Some(s)
            }
            Err(e) => {
                tracing::warn!("Could not bind discovery port {}: {}. Discovery will be limited to self-node only.", DISCOVERY_PORT, e);
                None
            }
        };

        let mut tick = interval(Duration::from_secs(ANNOUNCE_INTERVAL_SECS));
        loop {
            tick.tick().await;

            // Broadcast our presence
            if let Some(ref sock) = socket {
                let dest = format!("255.255.255.255:{}", DISCOVERY_PORT);
                match sock.send_to(announce_msg.as_bytes(), &dest) {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::trace!("Broadcast send error (non-fatal): {}", e);
                    }
                }
            }

            // Listen for announcements from other nodes
            if let Some(ref sock) = socket {
                let mut buf = [0u8; 512];
                // Drain all pending datagrams
                loop {
                    match sock.recv_from(&mut buf) {
                        Ok((len, addr)) => {
                            let msg = String::from_utf8_lossy(&buf[..len]);
                            if let Some(node) = parse_announce(&msg, &addr.ip().to_string()) {
                                if node.node_id != our_node_id {
                                    tracing::debug!(
                                        "Discovered mesh node: {} at {}",
                                        node.display_name,
                                        node.address
                                    );
                                    add_node(node);
                                }
                            }
                        }
                        Err(_) => break, // WouldBlock or other — no more datagrams
                    }
                }
            }

            // Keep self-node fresh
            {
                ensure_init();
                let mut guard = nodes_map().lock().unwrap();
                if let Some(map) = guard.as_mut() {
                    if let Some(self_n) = map.get_mut(&our_node_id) {
                        self_n.last_seen = chrono::Utc::now().to_rfc3339();
                    }
                }
            }

            // Remove stale remote nodes
            remove_stale_nodes();
        }
    });

    Ok(())
}

/// Stop discovery and deregister
pub fn stop_discovery() {
    let mut guard = nodes_map().lock().unwrap();
    if let Some(map) = guard.as_mut() {
        map.clear();
    }
    tracing::info!("Mesh discovery stopped");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_announce_valid() {
        let node = parse_announce("AGENTOS|abc123|MyPC|9090", "192.168.1.5");
        assert!(node.is_some());
        let n = node.unwrap();
        assert_eq!(n.node_id, "abc123");
        assert_eq!(n.display_name, "MyPC");
        assert_eq!(n.mesh_port, 9090);
        assert_eq!(n.address, "192.168.1.5:9090");
    }

    #[test]
    fn test_parse_announce_invalid() {
        assert!(parse_announce("GARBAGE|data", "1.2.3.4").is_none());
        assert!(parse_announce("AGENTOS|a|b", "1.2.3.4").is_none());
        assert!(parse_announce("OTHER|a|b|9090", "1.2.3.4").is_none());
    }

    #[test]
    fn test_node_registry() {
        ensure_init();
        let node = MeshNode {
            node_id: "test1".to_string(),
            display_name: "TestNode".to_string(),
            status: "online".to_string(),
            last_seen: chrono::Utc::now().to_rfc3339(),
            address: "10.0.0.1:9090".to_string(),
            capabilities: vec!["chat".to_string()],
            mesh_port: 9090,
        };
        add_node(node);
        let nodes = get_discovered_nodes();
        assert!(nodes.iter().any(|n| n.node_id == "test1"));
    }
}
