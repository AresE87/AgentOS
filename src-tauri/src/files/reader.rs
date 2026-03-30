use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FileContent {
    Text { content: String, line_count: usize },
    Table { headers: Vec<String>, rows: Vec<Vec<String>>, row_count: usize },
    Image { width: u32, height: u32, format: String, base64_preview: Option<String> },
    Binary { description: String, size_bytes: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePreview {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub content: FileContent,
}

pub struct FileReader;

impl FileReader {
    pub fn read(path: &Path) -> Result<FilePreview, String> {
        let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();

        let content = match ext.as_str() {
            "txt" | "md" | "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "json" | "yaml"
            | "yml" | "toml" | "xml" | "html" | "css" | "sql" | "sh" | "bat" | "ps1"
            | "log" | "cfg" | "ini" | "env" => {
                let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
                let lines = text.lines().count();
                // Truncate to 50KB for preview
                let truncated = if text.len() > 50_000 {
                    text[..50_000].to_string() + "\n...[truncated]"
                } else {
                    text
                };
                FileContent::Text {
                    content: truncated,
                    line_count: lines,
                }
            }
            "csv" | "tsv" => {
                let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
                let sep = if ext == "tsv" { '\t' } else { ',' };
                let mut lines = text.lines();
                let headers: Vec<String> = lines
                    .next()
                    .unwrap_or("")
                    .split(sep)
                    .map(|s| s.trim().to_string())
                    .collect();
                let rows: Vec<Vec<String>> = lines
                    .take(100)
                    .map(|line| line.split(sep).map(|s| s.trim().to_string()).collect())
                    .collect();
                let row_count = rows.len();
                FileContent::Table {
                    headers,
                    rows,
                    row_count,
                }
            }
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" => {
                let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
                let (w, h) = guess_image_dimensions(&bytes);
                let b64 = if bytes.len() < 500_000 {
                    Some(base64::engine::general_purpose::STANDARD.encode(&bytes))
                } else {
                    None
                };
                FileContent::Image {
                    width: w,
                    height: h,
                    format: ext.clone(),
                    base64_preview: b64,
                }
            }
            "pdf" => {
                let text = extract_pdf_text(path)?;
                let lines = text.lines().count();
                FileContent::Text {
                    content: text,
                    line_count: lines,
                }
            }
            "xlsx" | "xls" => {
                let (headers, rows) = extract_excel_data(path)?;
                let rc = rows.len();
                FileContent::Table {
                    headers,
                    rows,
                    row_count: rc,
                }
            }
            "docx" => {
                let text = extract_docx_text(path)?;
                let lines = text.lines().count();
                FileContent::Text {
                    content: text,
                    line_count: lines,
                }
            }
            _ => FileContent::Binary {
                description: format!("{} file", ext),
                size_bytes: metadata.len(),
            },
        };

        Ok(FilePreview {
            path: path.to_string_lossy().to_string(),
            name,
            extension: ext,
            size_bytes: metadata.len(),
            content,
        })
    }
}

use base64::Engine;

fn guess_image_dimensions(bytes: &[u8]) -> (u32, u32) {
    // PNG: width at byte 16, height at byte 20 (big-endian u32)
    if bytes.len() > 24 && bytes[0..4] == *b"\x89PNG" {
        let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        return (w, h);
    }
    // JPEG: search for SOF0 marker (0xFF 0xC0)
    if bytes.len() > 4 && bytes[0] == 0xFF && bytes[1] == 0xD8 {
        let mut i = 2;
        while i + 9 < bytes.len() {
            if bytes[i] == 0xFF && (bytes[i + 1] == 0xC0 || bytes[i + 1] == 0xC2) {
                let h = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
                let w = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
                return (w, h);
            }
            // Skip to next marker
            if bytes[i] == 0xFF && i + 3 < bytes.len() {
                let seg_len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
                i += 2 + seg_len;
            } else {
                i += 1;
            }
        }
    }
    (0, 0)
}

fn extract_pdf_text(path: &Path) -> Result<String, String> {
    let p = path.to_string_lossy();
    let script = format!(
        "try {{ \
            Add-Type -AssemblyName 'System.IO.Compression.FileSystem' -ErrorAction SilentlyContinue; \
            Write-Output '[PDF file: {}]' \
        }} catch {{ Write-Output '[PDF file - use vision mode for analysis]' }}",
        p.replace('\'', "''")
    );
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string())
}

fn extract_excel_data(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let p = path.to_string_lossy();
    let script = format!(
        "$excel = New-Object -ComObject Excel.Application -ErrorAction Stop; \
         $excel.Visible = $false; \
         $wb = $excel.Workbooks.Open('{}'); \
         $ws = $wb.Sheets.Item(1); \
         $used = $ws.UsedRange; \
         $rows = @(); \
         for ($r = 1; $r -le [Math]::Min($used.Rows.Count, 101); $r++) {{ \
             $row = @(); \
             for ($c = 1; $c -le [Math]::Min($used.Columns.Count, 20); $c++) {{ \
                 $row += $ws.Cells.Item($r, $c).Text; \
             }}; \
             $rows += ,($row -join '|||'); \
         }}; \
         $wb.Close($false); $excel.Quit(); \
         $rows -join \"`n\"",
        p.replace('\'', "''")
    );
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let mut lines = text.lines();
    let headers: Vec<String> = lines
        .next()
        .unwrap_or("")
        .split("|||")
        .map(|s| s.trim().to_string())
        .collect();
    let rows: Vec<Vec<String>> = lines
        .map(|l| l.split("|||").map(|s| s.trim().to_string()).collect())
        .collect();
    Ok((headers, rows))
}

fn extract_docx_text(path: &Path) -> Result<String, String> {
    // DOCX is a ZIP with word/document.xml
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let result = archive.by_name("word/document.xml");
    if let Ok(mut doc) = result {
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut doc, &mut xml).map_err(|e| e.to_string())?;
        // Strip XML tags to get plain text
        let text = xml
            .split('<')
            .filter_map(|s| {
                if s.starts_with("w:t") || s.starts_with("w:t ") {
                    s.split('>').nth(1)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        Ok(if text.is_empty() {
            "[DOCX - no text extracted]".into()
        } else {
            text
        })
    } else {
        Ok("[DOCX - could not read document.xml]".into())
    }
}
