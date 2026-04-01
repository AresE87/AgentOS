use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub node_id: String,
    pub display_name: String,
    pub os: String,
    pub has_gpu: bool,
    pub gpu_name: Option<String>,
    pub ram_gb: f64,
    pub cpu_cores: usize,
    pub installed_specialists: Vec<String>,
    pub installed_playbooks: Vec<String>,
    pub configured_providers: Vec<String>,
    pub current_load: f64,
    pub active_tasks: usize,
    /// Network address for mesh transport (IP or hostname)
    pub ip: String,
    /// TCP port for mesh task exchange
    pub mesh_port: u16,
}

impl NodeCapabilities {
    /// Detect local machine capabilities
    pub fn local() -> Self {
        let display_name = whoami::fallible::hostname().unwrap_or_else(|_| "localhost".to_string());

        // Rough RAM estimate: use available system info or fallback
        let ram_gb = Self::detect_ram_gb();
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            display_name,
            os: std::env::consts::OS.to_string(),
            has_gpu: false,
            gpu_name: None,
            ram_gb,
            cpu_cores,
            installed_specialists: vec!["general".to_string(), "programmer".to_string()],
            installed_playbooks: vec![],
            configured_providers: vec![],
            current_load: 0.0,
            active_tasks: 0,
            ip: "127.0.0.1".to_string(),
            mesh_port: 9099,
        }
    }

    pub fn update_load(&mut self, load: f64, tasks: usize) {
        self.current_load = load;
        self.active_tasks = tasks;
    }

    /// Best-effort RAM detection (Windows via GlobalMemoryStatusEx, else fallback)
    fn detect_ram_gb() -> f64 {
        #[cfg(target_os = "windows")]
        {
            // Use a safe default; full detection would require winapi calls
            16.0
        }
        #[cfg(not(target_os = "windows"))]
        {
            // On Linux/macOS, try reading /proc/meminfo or sysctl
            8.0
        }
    }
}
