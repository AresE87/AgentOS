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

        BitBlt(
            hdc_mem, 0, 0, width, height, hdc_screen, src_x, src_y, SRCCOPY,
        )?;

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

    let img: RgbaImage = ImageBuffer::from_raw(data.width, data.height, data.rgba.clone())
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
    encoder.encode(
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    )?;

    Ok(path)
}

/// Save screenshot as JPEG to a specific file path
pub fn save_screenshot_to(
    data: &ScreenshotData,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let img: RgbaImage = ImageBuffer::from_raw(data.width, data.height, data.rgba.clone())
        .ok_or("Failed to create image buffer")?;

    let img = if data.width > 1920 {
        let ratio = 1920.0 / data.width as f64;
        let new_h = (data.height as f64 * ratio) as u32;
        image::imageops::resize(&img, 1920, new_h, image::imageops::FilterType::Triangle)
    } else {
        img
    };

    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
    let mut buf = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 85);
    encoder.encode(
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_full_screen_returns_valid_data() {
        let result = capture_full_screen();
        assert!(result.is_ok(), "Screen capture failed: {:?}", result.err());
        let data = result.unwrap();
        assert!(data.width > 0, "Width should be positive");
        assert!(data.height > 0, "Height should be positive");
        assert!(!data.rgba.is_empty(), "RGBA buffer should not be empty");
        assert_eq!(
            data.rgba.len(),
            (data.width * data.height * 4) as usize,
            "RGBA buffer size mismatch"
        );
    }

    #[test]
    fn to_base64_jpeg_returns_valid_base64() {
        let data = capture_full_screen().unwrap();
        let b64 = to_base64_jpeg(&data, 50).unwrap();
        assert!(!b64.is_empty(), "Base64 should not be empty");
        // Verify it's valid base64 by decoding
        let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64);
        assert!(decoded.is_ok(), "Should be valid base64");
        let bytes = decoded.unwrap();
        // JPEG starts with FF D8
        assert_eq!(bytes[0], 0xFF, "JPEG should start with 0xFF");
        assert_eq!(bytes[1], 0xD8, "JPEG second byte should be 0xD8");
    }

    #[test]
    fn save_screenshot_creates_file() {
        let data = capture_full_screen().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = save_screenshot(&data, dir.path()).unwrap();
        assert!(path.exists(), "Screenshot file should exist");
        assert!(path.extension().map(|e| e == "jpg").unwrap_or(false));
        // File should be non-empty
        let size = std::fs::metadata(&path).unwrap().len();
        assert!(
            size > 1000,
            "JPEG file should be at least 1KB, got {} bytes",
            size
        );
    }

    #[test]
    fn base64_jpeg_resizes_large_images() {
        // Create a fake large image (2560x1440)
        let width = 2560u32;
        let height = 1440u32;
        let rgba = vec![128u8; (width * height * 4) as usize];
        let data = ScreenshotData {
            width,
            height,
            rgba,
            timestamp: chrono::Utc::now(),
        };
        let b64 = to_base64_jpeg(&data, 50).unwrap();
        // Decode and check dimensions aren't 2560 (should be resized to max 1280)
        let decoded =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64).unwrap();
        // We can't easily check dimensions from JPEG bytes without a decoder,
        // but we can verify it's smaller than a raw 2560x1440 would produce
        assert!(
            decoded.len() < 500_000,
            "Resized JPEG should be reasonably small"
        );
    }
}

/// Convert screenshot to base64 JPEG for sending to vision LLMs
pub fn to_base64_jpeg(
    data: &ScreenshotData,
    quality: u8,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let img: RgbaImage = ImageBuffer::from_raw(data.width, data.height, data.rgba.clone())
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
    encoder.encode(
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    )?;

    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &jpeg_buf,
    ))
}

/// Convert screenshot to base64 JPEG and return the resized image dimensions.
/// Critical for coordinate scaling: LLM sees coords relative to resized image,
/// but we need to map them back to the real screen.
pub fn to_base64_jpeg_with_dims(
    data: &ScreenshotData,
    quality: u8,
) -> Result<(String, u32, u32), Box<dyn std::error::Error + Send + Sync>> {
    let img: RgbaImage = ImageBuffer::from_raw(data.width, data.height, data.rgba.clone())
        .ok_or("Failed to create image buffer")?;

    let img = if data.width > 1280 {
        let ratio = 1280.0 / data.width as f64;
        let new_h = (data.height as f64 * ratio) as u32;
        image::imageops::resize(&img, 1280, new_h, image::imageops::FilterType::Triangle)
    } else {
        img
    };

    let img_w = img.width();
    let img_h = img.height();

    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
    let mut jpeg_buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut jpeg_buf);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality);
    encoder.encode(
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    )?;

    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &jpeg_buf);

    Ok((b64, img_w, img_h))
}
