pub mod traits;
pub mod windows;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "linux")]
pub mod linux;

pub use traits::PlatformProvider;

pub fn get_platform() -> Box<dyn PlatformProvider> {
    #[cfg(windows)]
    return Box::new(windows::WindowsPlatform::new());
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacosPlatform::new());
    #[cfg(target_os = "linux")]
    return Box::new(linux::LinuxPlatform::new());
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    panic!("Unsupported platform");
}
