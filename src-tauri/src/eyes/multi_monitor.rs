use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
    pub scale_factor: f64,
}

/// Detect all connected monitors
#[cfg(windows)]
pub fn detect_monitors() -> Vec<MonitorInfo> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CMONITORS, SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN,
        SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    };

    let mut monitors = Vec::new();

    unsafe {
        let num_monitors = GetSystemMetrics(SM_CMONITORS) as u32;
        let primary_w = GetSystemMetrics(SM_CXSCREEN) as u32;
        let primary_h = GetSystemMetrics(SM_CYSCREEN) as u32;
        let _virtual_x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let _virtual_y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let virtual_w = GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32;
        let virtual_h = GetSystemMetrics(SM_CYVIRTUALSCREEN) as u32;

        monitors.push(MonitorInfo {
            id: 0,
            name: "Primary".to_string(),
            x: 0,
            y: 0,
            width: primary_w,
            height: primary_h,
            is_primary: true,
            scale_factor: 1.0,
        });

        if num_monitors > 1 {
            // Approximate secondary monitor position (to the right of primary)
            let secondary_w = if virtual_w > primary_w {
                virtual_w - primary_w
            } else {
                primary_w
            };
            monitors.push(MonitorInfo {
                id: 1,
                name: "Secondary".to_string(),
                x: primary_w as i32,
                y: 0,
                width: secondary_w,
                height: virtual_h,
                is_primary: false,
                scale_factor: 1.0,
            });
        }
    }

    monitors
}

#[cfg(not(windows))]
pub fn detect_monitors() -> Vec<MonitorInfo> {
    vec![MonitorInfo {
        id: 0,
        name: "Primary".into(),
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        is_primary: true,
        scale_factor: 1.0,
    }]
}
