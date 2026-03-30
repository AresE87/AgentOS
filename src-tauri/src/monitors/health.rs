pub struct SystemHealthMonitor;

impl SystemHealthMonitor {
    pub async fn check() -> Option<(String, String, String)> {
        let output = tokio::process::Command::new("powershell")
            .args(&[
                "-NoProfile",
                "-Command",
                "$cpu = (Get-CimInstance Win32_Processor).LoadPercentage; $ram = Get-CimInstance Win32_OperatingSystem; $ramPct = [math]::Round((($ram.TotalVisibleMemorySize - $ram.FreePhysicalMemory) / $ram.TotalVisibleMemorySize) * 100, 0); Write-Output \"$cpu,$ramPct\"",
            ])
            .output()
            .await
            .ok()?;

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = text.split(',').collect();
        if parts.len() < 2 {
            return None;
        }
        let cpu: f64 = parts[0].parse().unwrap_or(0.0);
        let ram: f64 = parts[1].parse().unwrap_or(0.0);

        if cpu > 95.0 || ram > 95.0 {
            Some((
                "critical".into(),
                "System Under Heavy Load".into(),
                format!("CPU: {:.0}%, RAM: {:.0}%", cpu, ram),
            ))
        } else if cpu > 85.0 || ram > 85.0 {
            Some((
                "warning".into(),
                "High System Usage".into(),
                format!("CPU: {:.0}%, RAM: {:.0}%", cpu, ram),
            ))
        } else {
            None
        }
    }
}
