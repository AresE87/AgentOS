#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, VkKeyScanW, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
    KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT, VIRTUAL_KEY,
    VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_LEFT, VK_MENU,
    VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

/// Move mouse to absolute screen coordinates and click
pub fn click(x: i32, y: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(windows)]
    {
        mouse_move(x, y)?;
        std::thread::sleep(std::time::Duration::from_millis(30));
        mouse_button(MOUSEEVENTF_LEFTDOWN)?;
        std::thread::sleep(std::time::Duration::from_millis(30));
        mouse_button(MOUSEEVENTF_LEFTUP)?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (x, y);
        Err("Input simulation only supported on Windows".into())
    }
}

pub fn double_click(x: i32, y: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    click(x, y)?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    click(x, y)
}

pub fn right_click(x: i32, y: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(windows)]
    {
        mouse_move(x, y)?;
        std::thread::sleep(std::time::Duration::from_millis(30));
        mouse_button(MOUSEEVENTF_RIGHTDOWN)?;
        std::thread::sleep(std::time::Duration::from_millis(30));
        mouse_button(MOUSEEVENTF_RIGHTUP)?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (x, y);
        Err("Input simulation only supported on Windows".into())
    }
}

#[cfg(windows)]
fn mouse_move(x: i32, y: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    unsafe {
        let screen_w = GetSystemMetrics(SM_CXSCREEN) as f64;
        let screen_h = GetSystemMetrics(SM_CYSCREEN) as f64;
        // Normalize to 0..65535 range
        let norm_x = ((x as f64 / screen_w) * 65535.0) as i32;
        let norm_y = ((y as f64 / screen_h) * 65535.0) as i32;

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: norm_x,
                    dy: norm_y,
                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                    ..Default::default()
                },
            },
        };
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        Ok(())
    }
}

#[cfg(windows)]
fn mouse_button(
    flags: windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        };
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        Ok(())
    }
}

/// Scroll at the given position
pub fn scroll(x: i32, y: i32, delta: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(windows)]
    {
        mouse_move(x, y)?;
        std::thread::sleep(std::time::Duration::from_millis(30));
        unsafe {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        mouseData: (delta * 120) as u32, // 120 = one notch
                        dwFlags: MOUSEEVENTF_WHEEL,
                        ..Default::default()
                    },
                },
            };
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (x, y, delta);
        Err("Input simulation only supported on Windows".into())
    }
}

/// Type text character by character using Unicode input events
pub fn type_text(text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(windows)]
    {
        for ch in text.chars() {
            let utf16: Vec<u16> = ch.encode_utf16(&mut [0; 2]).to_vec();
            for code in utf16 {
                unsafe {
                    let down = INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wScan: code,
                                dwFlags: KEYEVENTF_UNICODE,
                                ..Default::default()
                            },
                        },
                    };
                    let up = INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wScan: code,
                                dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                                ..Default::default()
                            },
                        },
                    };
                    SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = text;
        Err("Input simulation only supported on Windows".into())
    }
}

/// Press a key combination (e.g., ["ctrl", "c"] for Ctrl+C)
pub fn key_combo(keys: &[String]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(windows)]
    {
        let vkeys: Vec<VIRTUAL_KEY> = keys.iter().map(|k| key_name_to_vk(k)).collect();

        // Press all keys down
        for vk in &vkeys {
            send_key(*vk, false)?;
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        // Release in reverse order
        for vk in vkeys.iter().rev() {
            send_key(*vk, true)?;
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = keys;
        Err("Input simulation only supported on Windows".into())
    }
}

#[cfg(windows)]
fn send_key(vk: VIRTUAL_KEY, up: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    unsafe {
        let flags = if up {
            KEYEVENTF_KEYUP
        } else {
            windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0)
        };

        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        };
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        Ok(())
    }
}

#[cfg(windows)]
fn key_name_to_vk(name: &str) -> VIRTUAL_KEY {
    match name.to_lowercase().as_str() {
        "ctrl" | "control" => VK_CONTROL,
        "shift" => VK_SHIFT,
        "alt" => VK_MENU,
        "enter" | "return" => VK_RETURN,
        "tab" => VK_TAB,
        "escape" | "esc" => VK_ESCAPE,
        "backspace" => VK_BACK,
        "delete" | "del" => VK_DELETE,
        "space" => VK_SPACE,
        "left" => VK_LEFT,
        "right" => VK_RIGHT,
        "up" => VK_UP,
        "down" => VK_DOWN,
        "home" => VK_HOME,
        "end" => VK_END,
        s if s.len() == 1 => {
            // Single character — map to VK
            let ch = s.chars().next().unwrap();
            let vk = unsafe { VkKeyScanW(ch as u16) };
            VIRTUAL_KEY((vk & 0xFF) as u16)
        }
        _ => VIRTUAL_KEY(0), // unknown key
    }
}
