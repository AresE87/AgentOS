use crate::types::{CaptureRegion, ScreenshotData};
use image::{ImageBuffer, Rgba, RgbaImage};
use std::path::{Path, PathBuf};

#[cfg(windows)]
use windows::Win32::Foundation::RECT;
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

/// Capture the full primary screen
pub fn capture_full_screen() -> Result<ScreenshotData, Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(windows)]
    {
        capture_screen_gdi(None)
    }
    #[cfg(not(windows))]
    {
        Err("Screen capture only supported on Windows".into())
    }
}

/// Capture a specific region of the screen
pub fn capture_region(
    region: &CaptureRegion,
) -> Result<ScreenshotData, Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(windows)]
    {
        capture_screen_gdi(Some(region))
    }
    #[cfg(not(windows))]
    {
        let _ = region;
        Err("Screen capture only supported on Windows".into())
    }
}

#[cfg(windows)]
fn capture_screen_gdi(
    region: Option<&CaptureRegion>,
) -> Result<ScreenshotData, Box<dyn std::error::Error + Send + Sync>> {
    unsafe {
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);

        let (src_x, src_y, width, height) = match region {
            Some(r) => (r.x, r.y, r.width as i32, r.height as i32),
            None => (0, 0, screen_w, screen_h),
        };

        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbm = CreateCompatibleBitmap(hdc_screen, width, height);
        let old = SelectObject(hdc_mem, hbm);

        BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, src_x, src_y, SRCCOPY)?;

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        GetDIBits(
            hdc_mem,
            hbm,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // BGRA → RGBA
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        SelectObject(hdc_mem, old);
        let _ = DeleteObject(hbm);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);

        Ok(ScreenshotData {
            width: width as u32,
            height: height as u32,
            rgba: pixels,
            timestamp: chrono::Utc::now(),
        })
    }
}

/// Save screenshot as JPEG to disk (lighter than WebP, widely compatible)
pub fn save_screenshot(
    data: &ScreenshotData,
    dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    std::fs::create_dir_all(dir)?;
    let ts = data.timestamp.format("%Y%m%d_%H%M%S_%3f");
    let short_id = &uuid::Uuid::new_v4().to_string()[..8];
    let filename = format!("{}_{}.jpg", ts, short_id);
    let path = dir.join(&filename);

    let img: RgbaImage =
        ImageBuffer::from_raw(data.width, data.height, data.rgba.clone())
            .ok_or("Failed to create image buffer")?;

    // Resize if very large (for storage efficiency)
    let img = if data.width > 1920 {
        let ratio = 1920.0 / data.width as f64;
        let new_h = (data.height as f64 * ratio) as u32;
        image::imageops::resize(&img, 1920, new_h, image::imageops::FilterType::Triangle)
    } else {
        img
    };

    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
    let mut buf = std::io::BufWriter::new(std::fs::File::create(&path)?);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 85);
    encoder.encode(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)?;

    Ok(path)
}

/// Convert screenshot to base64 JPEG for sending to vision LLMs
pub fn to_base64_jpeg(
    data: &ScreenshotData,
    quality: u8,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let img: RgbaImage =
        ImageBuffer::from_raw(data.width, data.height, data.rgba.clone())
            .ok_or("Failed to create image buffer")?;

    // Resize to max 1280px width for token efficiency
    let img = if data.width > 1280 {
        let ratio = 1280.0 / data.width as f64;
        let new_h = (data.height as f64 * ratio) as u32;
        image::imageops::resize(&img, 1280, new_h, image::imageops::FilterType::Triangle)
    } else {
        img
    };

    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
    let mut jpeg_buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut jpeg_buf);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality);
    encoder.encode(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)?;

    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &jpeg_buf,
    ))
}
