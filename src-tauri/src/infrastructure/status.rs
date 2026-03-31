use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionStatus {
    pub region: String,
    pub status: String,
    pub latency_ms: u32,
    pub last_checked: String,
    pub probe_type: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfraStatus {
    pub regions: Vec<RegionStatus>,
    pub global_status: String,
    pub uptime_pct: f64,
    pub probe_mode: String,
    pub source_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeTarget {
    pub region: String,
    pub endpoint: String,
    pub probe_type: String,
}

pub struct InfraMonitor;

impl InfraMonitor {
    pub fn new() -> Self {
        Self
    }

    pub fn check_regions(&self, probes: &[ProbeTarget]) -> InfraStatus {
        if probes.is_empty() {
            return InfraStatus {
                regions: Vec::new(),
                global_status: "no_configured_probes".to_string(),
                uptime_pct: 0.0,
                probe_mode: "none".to_string(),
                source_note: "No remote or local probe targets are configured.".to_string(),
            };
        }

        let now = chrono::Utc::now().to_rfc3339();
        let regions = probes
            .iter()
            .map(|probe| match probe.probe_type.as_str() {
                "file_exists" => probe_file_exists(probe, &now),
                _ => probe_tcp_endpoint(probe, &now),
            })
            .collect::<Vec<_>>();

        let operational = regions.iter().filter(|region| region.status == "operational").count();
        let any_down = regions.iter().any(|region| region.status == "down");
        let global_status = if operational == regions.len() {
            "operational".to_string()
        } else if any_down {
            "degraded".to_string()
        } else {
            "partial".to_string()
        };

        InfraStatus {
            regions,
            global_status,
            uptime_pct: 0.0,
            probe_mode: "real_probe_snapshot".to_string(),
            source_note: "Current snapshot from live probes. No rolling uptime history is stored yet.".to_string(),
        }
    }

    pub fn get_status_page_data(&self, probes: &[ProbeTarget]) -> serde_json::Value {
        let status = self.check_regions(probes);
        serde_json::json!({
            "page": {
                "name": "AgentOS Status",
                "updated_at": chrono::Utc::now().to_rfc3339(),
            },
            "status": {
                "indicator": status.global_status,
                "description": status.source_note,
                "probe_mode": status.probe_mode,
            },
            "components": status.regions,
            "uptime": {
                "snapshot_only": true,
                "last_30_days": status.uptime_pct,
            },
        })
    }
}

fn probe_file_exists(probe: &ProbeTarget, now: &str) -> RegionStatus {
    let path = Path::new(&probe.endpoint);
    RegionStatus {
        region: probe.region.clone(),
        status: if path.exists() { "operational" } else { "down" }.to_string(),
        latency_ms: 0,
        last_checked: now.to_string(),
        probe_type: probe.probe_type.clone(),
        note: format!("Filesystem probe against {}", probe.endpoint),
    }
}

fn probe_tcp_endpoint(probe: &ProbeTarget, now: &str) -> RegionStatus {
    let started_at = Instant::now();
    let addr = parse_socket_addr(&probe.endpoint);

    match addr.and_then(|socket_addr| TcpStream::connect_timeout(&socket_addr, Duration::from_secs(2)).ok()) {
        Some(_) => RegionStatus {
            region: probe.region.clone(),
            status: "operational".to_string(),
            latency_ms: started_at.elapsed().as_millis() as u32,
            last_checked: now.to_string(),
            probe_type: probe.probe_type.clone(),
            note: format!("TCP probe against {}", probe.endpoint),
        },
        None => RegionStatus {
            region: probe.region.clone(),
            status: "down".to_string(),
            latency_ms: started_at.elapsed().as_millis() as u32,
            last_checked: now.to_string(),
            probe_type: probe.probe_type.clone(),
            note: format!("TCP probe failed for {}", probe.endpoint),
        },
    }
}

fn parse_socket_addr(endpoint: &str) -> Option<SocketAddr> {
    let trimmed = endpoint
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/');

    if let Ok(socket_addr) = trimmed.parse::<SocketAddr>() {
        return Some(socket_addr);
    }

    let host_port = trimmed.split('/').next()?;
    let mut parts = host_port.split(':');
    let host = parts.next()?;
    let port = parts.next()?.parse::<u16>().ok()?;
    format!("{}:{}", host, port).parse::<SocketAddr>().ok()
}
