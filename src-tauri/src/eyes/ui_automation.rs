use crate::types::{UIElement, WindowInfo};

#[cfg(windows)]
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
#[cfg(windows)]
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};
#[cfg(windows)]
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, TreeScope_Children,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
    IsWindowVisible,
};

/// Get UI elements of the foreground window
pub fn get_foreground_elements() -> Result<Vec<UIElement>, Box<dyn std::error::Error + Send + Sync>>
{
    #[cfg(windows)]
    {
        get_elements_windows()
    }
    #[cfg(not(windows))]
    {
        Err("UI Automation only supported on Windows".into())
    }
}

#[cfg(windows)]
fn get_elements_windows() -> Result<Vec<UIElement>, Box<dyn std::error::Error + Send + Sync>> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();

        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?;
        let root = automation.GetFocusedElement()?;

        let elements = read_children(&automation, &root, 0, 3)?;

        CoUninitialize();
        Ok(elements)
    }
}

#[cfg(windows)]
unsafe fn read_children(
    automation: &IUIAutomation,
    element: &IUIAutomationElement,
    depth: u32,
    max_depth: u32,
) -> Result<Vec<UIElement>, Box<dyn std::error::Error + Send + Sync>> {
    if depth >= max_depth {
        return Ok(vec![]);
    }

    let mut results = Vec::new();
    let condition = automation.CreateTrueCondition()?;
    let children = element.FindAll(TreeScope_Children, &condition)?;
    let count = children.Length()?;

    for i in 0..count.min(50) {
        // Cap at 50 children per level
        if let Ok(child) = children.GetElement(i) {
            let name = child
                .CurrentName()
                .map(|s| s.to_string())
                .unwrap_or_default();
            let control_type = child.CurrentControlType().map(|c| c.0).unwrap_or(0);
            let automation_id = child
                .CurrentAutomationId()
                .map(|s| s.to_string())
                .unwrap_or_default();
            let is_enabled = child.CurrentIsEnabled().unwrap_or(BOOL(0)).as_bool();

            let rect = child.CurrentBoundingRectangle().unwrap_or(RECT::default());
            let bounding_rect = (
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
            );

            let sub_children =
                read_children(automation, &child, depth + 1, max_depth).unwrap_or_default();

            results.push(UIElement {
                name,
                control_type: control_type_name(control_type),
                automation_id,
                bounding_rect,
                is_enabled,
                value: None,
                children: sub_children,
            });
        }
    }

    Ok(results)
}

#[cfg(windows)]
fn control_type_name(id: i32) -> String {
    match id {
        50000 => "Button",
        50002 => "CheckBox",
        50003 => "ComboBox",
        50004 => "Edit",
        50005 => "Hyperlink",
        50006 => "Image",
        50007 => "ListItem",
        50008 => "List",
        50009 => "Menu",
        50010 => "MenuBar",
        50011 => "MenuItem",
        50016 => "RadioButton",
        50018 => "ScrollBar",
        50020 => "StaticText",
        50021 => "StatusBar",
        50022 => "Tab",
        50023 => "TabItem",
        50025 => "ToolBar",
        50026 => "ToolTip",
        50027 => "Tree",
        50028 => "TreeItem",
        50032 => "Window",
        50033 => "Pane",
        50037 => "Document",
        50040 => "TitleBar",
        _ => "Unknown",
    }
    .to_string()
}

/// List all visible top-level windows
pub fn list_windows() -> Result<Vec<WindowInfo>, Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(windows)]
    {
        list_windows_win32()
    }
    #[cfg(not(windows))]
    {
        Err("Window listing only supported on Windows".into())
    }
}

#[cfg(windows)]
fn list_windows_win32() -> Result<Vec<WindowInfo>, Box<dyn std::error::Error + Send + Sync>> {
    use std::sync::Mutex;
    let windows: std::sync::Arc<Mutex<Vec<WindowInfo>>> =
        std::sync::Arc::new(Mutex::new(Vec::new()));
    let windows_clone = windows.clone();

    unsafe {
        EnumWindows(
            Some(enum_window_proc),
            LPARAM(&*windows_clone as *const Mutex<Vec<WindowInfo>> as isize),
        )?;
    }

    let result = windows.lock().unwrap().clone();
    Ok(result)
}

#[cfg(windows)]
unsafe extern "system" fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1); // continue
    }

    let title_len = GetWindowTextLengthW(hwnd);
    if title_len == 0 {
        return BOOL(1);
    }

    let mut title_buf = vec![0u16; (title_len + 1) as usize];
    GetWindowTextW(hwnd, &mut title_buf);
    let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

    let mut class_buf = [0u16; 256];
    let class_len = GetClassNameW(hwnd, &mut class_buf);
    let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);

    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);

    let info = WindowInfo {
        hwnd: hwnd.0 as isize,
        title,
        class_name,
        rect: (
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
        ),
        is_visible: true,
    };

    let windows = &*(lparam.0 as *const std::sync::Mutex<Vec<WindowInfo>>);
    if let Ok(mut w) = windows.lock() {
        w.push(info);
    }

    BOOL(1) // continue enumeration
}
