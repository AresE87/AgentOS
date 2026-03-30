pub struct DiskMonitor;

impl DiskMonitor {
    pub async fn check() -> Option<(String, String, String)> {
        // Returns (severity, title, message) if alert needed
        let output = tokio::process::Command::new("powershell")
            .args(&[
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_LogicalDisk -Filter \"DeviceID='C:'\" | Select-Object @{N='Free';E={[math]::Round($_.FreeSpace/1GB,1)}}, @{N='Total';E={[math]::Round($_.Size/1GB,1)}} | ConvertTo-Json",
            ])
            .output()
            .await
            .ok()?;

        let text = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&text).ok()?;
        let free = json.get("Free")?.as_f64()?;
        let total = json.get("Total")?.as_f64()?;
        let used_pct = ((total - free) / total) * 100.0;

        if used_pct > 95.0 {
            Some((
                "critical".into(),
                "Disk Almost Full".into(),
                format!("C: drive is {:.0}% full ({:.1} GB free)", used_pct, free),
            ))
        } else if used_pct > 85.0 {
            Some((
                "warning".into(),
                "Disk Space Low".into(),
                format!("C: drive is {:.0}% full ({:.1} GB free)", used_pct, free),
            ))
        } else {
            None
        }
    }
}
