use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshNode {
    pub node_id: String,
    pub display_name: String,
    pub status: String,
    pub last_seen: String,
    pub address: String,
    pub capabilities: Vec<String>,
}

static DISCOVERED_NODES: Mutex<Vec<MeshNode>> = Mutex::new(Vec::new());

/// Get the current list of discovered nodes
pub fn get_discovered_nodes() -> Vec<MeshNode> {
    DISCOVERED_NODES.lock().unwrap().clone()
}

/// Start mDNS discovery (runs in background)
/// In Phase 5 full implementation, this will use mdns-sd crate
pub async fn start_discovery(node_name: &str) -> Result<(), String> {
    tracing::info!("Mesh discovery started for node: {}", node_name);

    // Register ourselves
    let self_node = MeshNode {
        node_id: uuid::Uuid::new_v4().to_string(),
        display_name: node_name.to_string(),
        status: "online".to_string(),
        last_seen: chrono::Utc::now().to_rfc3339(),
        address: "127.0.0.1:9090".to_string(),
        capabilities: vec![
            "chat".to_string(),
            "screen".to_string(),
            "cli".to_string(),
        ],
    };

    if let Ok(mut nodes) = DISCOVERED_NODES.lock() {
        nodes.push(self_node);
    }

    // TODO: In full implementation, use mdns-sd to:
    // 1. Register _agentos._tcp.local service
    // 2. Browse for other _agentos._tcp.local services
    // 3. Update DISCOVERED_NODES when peers are found
    // 4. Heartbeat check every 10s, mark offline if 3 missed

    Ok(())
}

/// Stop discovery and deregister
pub fn stop_discovery() {
    if let Ok(mut nodes) = DISCOVERED_NODES.lock() {
        nodes.clear();
    }
    tracing::info!("Mesh discovery stopped");
}
