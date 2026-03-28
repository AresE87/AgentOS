use log::{error, info, warn};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use std::path::PathBuf;

pub struct PythonProcess {
    child: Option<Child>,
    request_id: u64,
    pending: Arc<(Mutex<HashMap<u64, Option<Result<Value, String>>>>, Condvar)>,
    stdin: Option<std::process::ChildStdin>,
    max_retries: u32,
    retry_count: u32,
}

impl PythonProcess {
    pub fn new() -> Self {
        Self {
            child: None,
            request_id: 0,
            pending: Arc::new((Mutex::new(HashMap::new()), Condvar::new())),
            stdin: None,
            max_retries: 3,
            retry_count: 0,
        }
    }

    fn find_project_root() -> PathBuf {
        // Look for the project root containing the agentos/ package
        // Try: next to exe, parent of exe, known dev path, current dir
        let exe_dir = std::env::current_exe()
            .map(|p| p.parent().unwrap_or(&PathBuf::from(".")).to_path_buf())
            .unwrap_or_else(|_| PathBuf::from("."));

        let candidates = [
            exe_dir.join("resources"),          // Bundled: resources/agentos/
            exe_dir.clone(),                     // Same dir as exe
            exe_dir.parent().unwrap_or(&exe_dir).to_path_buf(), // Parent of exe
            PathBuf::from(r"C:\Users\AresE\Documents\AgentOS"), // Dev path
            PathBuf::from("."),                  // Current dir
        ];

        for dir in &candidates {
            if dir.join("agentos").join("__init__.py").exists() {
                return dir.clone();
            }
        }

        // Fallback: check if agentos is pip-installed (importable from anywhere)
        PathBuf::from(".")
    }

    fn find_python() -> String {
        // Try bundled python first, then system python
        let exe_dir = std::env::current_exe()
            .map(|p| p.parent().unwrap_or(&PathBuf::from(".")).to_path_buf())
            .unwrap_or_else(|_| PathBuf::from("."));
        
        let bundled = exe_dir.join("resources").join("python").join("python.exe");
        if bundled.exists() {
            return bundled.to_string_lossy().to_string();
        }
        "python".to_string()
    }

    pub fn start(&mut self) -> Result<(), String> {
        info!("Starting Python IPC server...");
        let python = Self::find_python();
        info!("Using Python: {}", python);

        // Find the project root (where agentos/ package lives)
        let project_root = Self::find_project_root();
        info!("Project root: {:?}", project_root);

        let mut cmd = Command::new(&python);
        cmd.args(["-m", "agentos.ipc_server"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .current_dir(&project_root);

        // Set PYTHONPATH so Python finds the agentos package
        if let Some(existing) = std::env::var_os("PYTHONPATH") {
            let mut paths = std::env::split_paths(&existing).collect::<Vec<_>>();
            paths.insert(0, project_root.clone());
            cmd.env("PYTHONPATH", std::env::join_paths(paths).unwrap());
        } else {
            cmd.env("PYTHONPATH", &project_root);
        }

        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn Python ({}): {}", python, e))?;

        let stdout = child.stdout.take().ok_or("Failed to take stdout")?;
        self.stdin = child.stdin.take();
        self.child = Some(child);

        let pending = Arc::clone(&self.pending);
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(text) if !text.trim().is_empty() => {
                        if let Ok(msg) = serde_json::from_str::<Value>(&text) {
                            if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
                                let (lock, cvar) = &*pending;
                                let mut map = lock.lock().unwrap();
                                let result = if msg.get("error").is_some() {
                                    Err(msg["error"]["message"].as_str().unwrap_or("Error").to_string())
                                } else {
                                    Ok(msg["result"].clone())
                                };
                                map.insert(id, Some(result));
                                cvar.notify_all();
                            } else {
                                info!("Python event: {text}");
                            }
                        }
                    }
                    Err(e) => { error!("Stdout read error: {e}"); break; }
                    _ => {}
                }
            }
        });

        self.retry_count = 0;
        info!("Python IPC server started");
        Ok(())
    }

    pub fn send_request_sync(&mut self, method: &str, params: Value) -> Result<Value, String> {
        if !self.is_alive() {
            warn!("Python not alive, restarting...");
            self.restart()?;
        }
        self.request_id += 1;
        let id = self.request_id;
        let request = serde_json::json!({"jsonrpc":"2.0","method":method,"params":params,"id":id});
        {
            let (lock, _) = &*self.pending;
            lock.lock().unwrap().insert(id, None);
        }
        let stdin = self.stdin.as_mut().ok_or("Python not started")?;
        let line = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        writeln!(stdin, "{line}").map_err(|e| format!("Write failed: {e}"))?;
        stdin.flush().map_err(|e| format!("Flush failed: {e}"))?;

        let (lock, cvar) = &*self.pending;
        let mut map = lock.lock().unwrap();
        let timeout = Duration::from_secs(10);
        loop {
            if let Some(Some(result)) = map.remove(&id) { return result; }
            map.insert(id, None);
            let (new_map, wait_result) = cvar.wait_timeout(map, timeout).unwrap();
            map = new_map;
            if wait_result.timed_out() { map.remove(&id); return Err("Request timed out".to_string()); }
            if let Some(Some(result)) = map.remove(&id) { return result; }
        }
    }

    pub fn is_alive(&mut self) -> bool {
        self.child.as_mut().map_or(false, |c| c.try_wait().ok().flatten().is_none())
    }

    fn restart(&mut self) -> Result<(), String> {
        if self.retry_count >= self.max_retries { return Err("Python crashed too many times".to_string()); }
        self.retry_count += 1;
        warn!("Restarting Python ({}/{})", self.retry_count, self.max_retries);
        self.kill();
        self.start()
    }

    pub fn kill_sync(&mut self) { self.kill(); }

    fn kill(&mut self) {
        if let Some(ref mut child) = self.child { let _ = child.kill(); let _ = child.wait(); }
        self.child = None;
        self.stdin = None;
    }
}

impl Drop for PythonProcess { fn drop(&mut self) { self.kill(); } }
