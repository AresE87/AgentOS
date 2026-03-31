use reqwest::Client;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::info;

const MAX_CONTENT_LENGTH: usize = 8000;
const PAGE_TIMEOUT_SECS: u64 = 15;

/// Domains that should never be fetched (local/internal networks)
const BLOCKED_PATTERNS: &[&str] = &[
    "localhost",
    "127.0.0.1",
    "0.0.0.0",
    "192.168.",
    "10.",
    "172.16.",
    "172.17.",
    "172.18.",
    "172.19.",
    "172.20.",
    "172.21.",
    "172.22.",
    "172.23.",
    "172.24.",
    "172.25.",
    "172.26.",
    "172.27.",
    "172.28.",
    "172.29.",
    "172.30.",
    "172.31.",
    "[::1]",
    "169.254.",
];

#[derive(Debug, Serialize)]
pub struct PageContent {
    pub url: String,
    pub title: String,
    pub text: String,
    pub status: u16,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub title: String,
    pub snippet: String,
    pub url: String,
}

/// Check if a URL is safe to fetch (not internal/local)
pub fn is_url_safe(url: &str) -> bool {
    let lower = url.to_lowercase();
    !BLOCKED_PATTERNS.iter().any(|p| lower.contains(p))
}

/// Fetch a URL and extract readable text content
pub async fn fetch_page(url: &str) -> Result<PageContent, String> {
    if !is_url_safe(url) {
        return Err(format!(
            "Blocked: URL '{}' points to a local/internal address",
            url
        ));
    }

    info!(url, "Fetching web page");

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(PAGE_TIMEOUT_SECS))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .get(url)
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .header("Accept-Language", "en-US,en;q=0.9,es;q=0.8")
        .send()
        .await
        .map_err(|e| format!("Request failed for '{}': {}", url, e))?;

    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if status >= 400 {
        return Err(format!("HTTP {} for '{}'", status, url));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let text = if content_type.contains("html") || content_type.contains("xhtml") {
        extract_text_from_html(&body)
    } else if content_type.contains("json") {
        // Pretty-print JSON for readability
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) {
            serde_json::to_string_pretty(&parsed)
                .unwrap_or_else(|_| body.chars().take(MAX_CONTENT_LENGTH).collect())
        } else {
            body.chars().take(MAX_CONTENT_LENGTH).collect()
        }
    } else {
        body.chars().take(MAX_CONTENT_LENGTH).collect()
    };

    let title = if content_type.contains("html") {
        extract_title(&body)
    } else {
        String::new()
    };

    info!(url, status, title = %title, text_len = text.len(), "Page fetched");

    Ok(PageContent {
        url: url.to_string(),
        title,
        text,
        status,
    })
}

/// Extract readable text from HTML, removing scripts, styles, and tags
fn extract_text_from_html(html: &str) -> String {
    let mut text = html.to_string();

    // Remove script and style blocks
    let script_re = regex::Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
    text = script_re.replace_all(&text, "").to_string();

    let style_re = regex::Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
    text = style_re.replace_all(&text, "").to_string();

    // Remove HTML comments
    let comment_re = regex::Regex::new(r"(?s)<!--.*?-->").unwrap();
    text = comment_re.replace_all(&text, "").to_string();

    // Remove head section (meta tags, links, etc.)
    let head_re = regex::Regex::new(r"(?is)<head[^>]*>.*?</head>").unwrap();
    text = head_re.replace_all(&text, "").to_string();

    // Remove nav and footer (often noisy)
    let nav_re = regex::Regex::new(r"(?is)<nav[^>]*>.*?</nav>").unwrap();
    text = nav_re.replace_all(&text, "").to_string();

    // Replace block-level tags with newlines for readability
    let block_re =
        regex::Regex::new(r"<(?:p|div|br|h[1-6]|li|tr|section|article|header|footer)[^>]*>")
            .unwrap();
    text = block_re.replace_all(&text, "\n").to_string();

    // Remove all remaining HTML tags
    let tag_re = regex::Regex::new(r"<[^>]+>").unwrap();
    text = tag_re.replace_all(&text, "").to_string();

    // Decode common HTML entities
    text = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&#x27;", "'")
        .replace("&#x2F;", "/")
        .replace("&mdash;", "\u{2014}")
        .replace("&ndash;", "\u{2013}")
        .replace("&hellip;", "\u{2026}")
        .replace("&laquo;", "\u{00AB}")
        .replace("&raquo;", "\u{00BB}");

    // Decode numeric HTML entities (&#NNN;)
    let num_entity_re = regex::Regex::new(r"&#(\d+);").unwrap();
    text = num_entity_re
        .replace_all(&text, |caps: &regex::Captures| {
            caps.get(1)
                .and_then(|m| m.as_str().parse::<u32>().ok())
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_default()
        })
        .to_string();

    // Collapse whitespace within lines
    let ws_re = regex::Regex::new(r"[ \t]+").unwrap();
    text = ws_re.replace_all(&text, " ").to_string();

    // Collapse multiple newlines
    let nl_re = regex::Regex::new(r"\n{3,}").unwrap();
    text = nl_re.replace_all(&text, "\n\n").to_string();

    // Trim each line
    text = text
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join("\n");

    // Collapse again after trimming
    let nl_re2 = regex::Regex::new(r"\n{3,}").unwrap();
    text = nl_re2.replace_all(&text, "\n\n").to_string();

    text.trim().chars().take(MAX_CONTENT_LENGTH).collect()
}

/// Extract the <title> from an HTML document
fn extract_title(html: &str) -> String {
    let title_re = regex::Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap();
    title_re
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| {
            let raw = m.as_str().trim();
            // Strip tags from title (some pages have tags inside <title>)
            let tag_re = regex::Regex::new(r"<[^>]+>").unwrap();
            tag_re.replace_all(raw, "").trim().to_string()
        })
        .unwrap_or_default()
}

/// Search the web using DuckDuckGo HTML endpoint
pub async fn web_search(query: &str) -> Result<Vec<SearchResult>, String> {
    info!(query, "Web search");

    // URL-encode the query manually (simple percent encoding)
    let encoded = simple_url_encode(query);
    let url = format!("https://html.duckduckgo.com/html/?q={}", encoded);

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(PAGE_TIMEOUT_SECS))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .header("Accept", "text/html")
        .send()
        .await
        .map_err(|e| format!("Search request failed: {}", e))?;

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read search response: {}", e))?;

    let results = parse_search_results(&body);
    info!(query, count = results.len(), "Search results parsed");
    Ok(results)
}

/// Parse DuckDuckGo HTML search results
fn parse_search_results(html: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // DuckDuckGo HTML results have class="result__a" for links and class="result__snippet" for snippets
    let link_re =
        regex::Regex::new(r#"(?is)<a[^>]*class="result__a"[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#)
            .unwrap();
    let snippet_re =
        regex::Regex::new(r#"(?is)<a[^>]*class="result__snippet"[^>]*>(.*?)</a>"#).unwrap();

    let links: Vec<_> = link_re.captures_iter(html).collect();
    let snippets: Vec<_> = snippet_re.captures_iter(html).collect();

    let tag_re = regex::Regex::new(r"<[^>]+>").unwrap();

    for (i, link_cap) in links.iter().enumerate().take(8) {
        let raw_url = link_cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let title_html = link_cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let title = tag_re.replace_all(title_html, "").trim().to_string();

        // DuckDuckGo wraps URLs through their redirect — extract the actual URL
        let url = if raw_url.contains("uddg=") {
            // Extract from redirect: //duckduckgo.com/l/?uddg=ENCODED_URL&...
            raw_url
                .split("uddg=")
                .nth(1)
                .and_then(|s| s.split('&').next())
                .map(|s| simple_url_decode(s))
                .unwrap_or_else(|| raw_url.to_string())
        } else {
            raw_url.to_string()
        };

        let snippet = snippets
            .get(i)
            .and_then(|c| c.get(1))
            .map(|m| tag_re.replace_all(m.as_str(), "").trim().to_string())
            .unwrap_or_default();

        if !title.is_empty() {
            results.push(SearchResult {
                title,
                snippet,
                url,
            });
        }
    }

    // Fallback: if regex didn't match the expected structure, try simpler extraction
    if results.is_empty() {
        let text = extract_text_from_html(html);
        for line in text.lines().take(50) {
            let trimmed = line.trim();
            if trimmed.len() > 30
                && !trimmed.starts_with("DuckDuckGo")
                && !trimmed.contains("Privacy")
            {
                results.push(SearchResult {
                    title: trimmed.chars().take(100).collect(),
                    snippet: trimmed.to_string(),
                    url: String::new(),
                });
                if results.len() >= 5 {
                    break;
                }
            }
        }
    }

    results
}

/// Simple percent-encoding for URL query parameters
fn simple_url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push_str(&format!("{:02X}", b));
            }
        }
    }
    result
}

/// Simple percent-decoding for URLs
fn simple_url_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            }
            '+' => result.push(' '),
            _ => result.push(c),
        }
    }
    result
}

// ── C10: Headless Browser (Chrome/Edge) ─────────────────────────

/// Known browser executable paths on Windows
const BROWSER_CANDIDATES: &[&str] = &[
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
];

#[derive(Debug, Serialize)]
pub struct BrowserInfo {
    pub available: bool,
    pub browser_path: Option<String>,
    pub browser_name: Option<String>,
}

/// Detect if Chrome or Edge is installed and return its path.
pub fn detect_browser() -> BrowserInfo {
    for candidate in BROWSER_CANDIDATES {
        let path = Path::new(candidate);
        if path.exists() {
            let name = if candidate.contains("chrome") {
                "Google Chrome"
            } else {
                "Microsoft Edge"
            };
            return BrowserInfo {
                available: true,
                browser_path: Some(candidate.to_string()),
                browser_name: Some(name.to_string()),
            };
        }
    }
    BrowserInfo {
        available: false,
        browser_path: None,
        browser_name: None,
    }
}

/// Fetch a page using headless Chrome/Edge (--headless --dump-dom).
/// This renders JavaScript before capturing the DOM, unlike plain reqwest.
/// Falls back to reqwest-based fetch_page() if no browser is found.
pub async fn fetch_with_browser(url: &str) -> Result<PageContent, String> {
    if !is_url_safe(url) {
        return Err(format!(
            "Blocked: URL '{}' points to a local/internal address",
            url
        ));
    }

    let browser = detect_browser();
    if !browser.available {
        info!(url, "No headless browser found, falling back to reqwest");
        return fetch_page(url).await;
    }

    let browser_path = browser.browser_path.unwrap();
    info!(url, browser = %browser_path, "Fetching with headless browser");

    let output = Command::new(&browser_path)
        .args([
            "--headless",
            "--disable-gpu",
            "--no-sandbox",
            "--dump-dom",
            url,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to launch headless browser: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Headless browser exited with error: {}",
            stderr.chars().take(500).collect::<String>()
        ));
    }

    let html = String::from_utf8_lossy(&output.stdout).to_string();
    let title = extract_title(&html);
    let text = extract_text_from_html(&html);

    info!(url, title = %title, text_len = text.len(), "Headless browser fetch complete");

    Ok(PageContent {
        url: url.to_string(),
        title,
        text,
        status: 200,
    })
}

/// Take a screenshot of a URL using headless Chrome/Edge.
/// Returns the path to the saved screenshot PNG.
pub async fn screenshot_url(url: &str, output_path: &Path) -> Result<PathBuf, String> {
    if !is_url_safe(url) {
        return Err(format!(
            "Blocked: URL '{}' points to a local/internal address",
            url
        ));
    }

    let browser = detect_browser();
    if !browser.available {
        return Err("No Chrome or Edge browser found for screenshots".to_string());
    }

    let browser_path = browser.browser_path.unwrap();
    let screenshot_arg = format!("--screenshot={}", output_path.display());

    info!(url, output = %output_path.display(), browser = %browser_path, "Taking screenshot with headless browser");

    let output = Command::new(&browser_path)
        .args([
            "--headless",
            "--disable-gpu",
            "--no-sandbox",
            "--window-size=1280,800",
            &screenshot_arg,
            url,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to launch headless browser for screenshot: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Screenshot failed: {}",
            stderr.chars().take(500).collect::<String>()
        ));
    }

    if output_path.exists() {
        info!(url, path = %output_path.display(), "Screenshot saved");
        Ok(output_path.to_path_buf())
    } else {
        Err("Screenshot file was not created by browser".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_url_safe() {
        assert!(is_url_safe("https://example.com"));
        assert!(is_url_safe("https://www.google.com/search?q=test"));
        assert!(!is_url_safe("http://localhost:8080"));
        assert!(!is_url_safe("http://127.0.0.1/admin"));
        assert!(!is_url_safe("http://192.168.1.1/router"));
        assert!(!is_url_safe("http://10.0.0.1/internal"));
        assert!(!is_url_safe("http://172.16.0.1/private"));
    }

    #[test]
    fn test_extract_title() {
        assert_eq!(
            extract_title("<html><head><title>Hello World</title></head></html>"),
            "Hello World"
        );
        assert_eq!(
            extract_title("<html><head><TITLE>  Trimmed  </TITLE></head></html>"),
            "Trimmed"
        );
        assert_eq!(extract_title("<html><head></head></html>"), "");
    }

    #[test]
    fn test_extract_text_from_html() {
        let html = r#"<html>
            <head><title>Test</title><style>body{color:red}</style></head>
            <body>
                <script>alert('hi')</script>
                <h1>Hello</h1>
                <p>This is a <b>test</b> paragraph.</p>
                <div>Another &amp; section</div>
            </body>
        </html>"#;
        let text = extract_text_from_html(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("test paragraph"));
        assert!(text.contains("Another & section"));
        assert!(!text.contains("alert"));
        assert!(!text.contains("color:red"));
    }

    #[test]
    fn test_simple_url_encode() {
        assert_eq!(simple_url_encode("hello world"), "hello+world");
        assert_eq!(simple_url_encode("a&b=c"), "a%26b%3Dc");
        assert_eq!(simple_url_encode("test"), "test");
    }

    #[test]
    fn test_simple_url_decode() {
        assert_eq!(simple_url_decode("hello+world"), "hello world");
        assert_eq!(simple_url_decode("a%26b%3Dc"), "a&b=c");
        assert_eq!(
            simple_url_decode("https%3A%2F%2Fexample.com"),
            "https://example.com"
        );
    }

    #[test]
    fn test_content_length_limit() {
        let long_html = format!("<html><body>{}</body></html>", "a".repeat(20000));
        let text = extract_text_from_html(&long_html);
        assert!(text.len() <= MAX_CONTENT_LENGTH);
    }
}
