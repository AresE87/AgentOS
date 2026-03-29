/// Sanitize string input — remove control characters, limit length
pub fn sanitize_input(input: &str, max_length: usize) -> String {
    input
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
        .take(max_length)
        .collect()
}

/// Sanitize for use in file paths — remove traversal patterns
pub fn sanitize_path(input: &str) -> String {
    input
        .replace("..", "")
        .replace("~", "")
        .replace("\\\\", "\\")
        .chars()
        .filter(|c| {
            c.is_alphanumeric()
                || *c == '/'
                || *c == '\\'
                || *c == '.'
                || *c == '_'
                || *c == '-'
                || *c == ' '
                || *c == ':'
        })
        .collect()
}

/// Sanitize for use in SQL (basic — rusqlite uses parameterized queries, but just in case)
pub fn sanitize_sql_value(input: &str) -> String {
    input.replace('\'', "''").replace(';', "").replace("--", "")
}

/// Validate API key format
pub fn validate_api_key_format(key: &str) -> bool {
    key.starts_with("aos_")
        && key.len() >= 36
        && key.chars().all(|c| c.is_ascii_hexdigit() || c == '_')
}

/// Sanitize output before sending to frontend (remove potential XSS)
pub fn sanitize_output(output: &str) -> String {
    output.replace('<', "&lt;").replace('>', "&gt;")
}

/// Check for common injection patterns
pub fn detect_injection(input: &str) -> Option<String> {
    let lower = input.to_lowercase();

    let patterns = [
        ("script>", "Potential XSS"),
        ("javascript:", "Potential XSS"),
        ("onerror=", "Potential XSS"),
        ("onload=", "Potential XSS"),
        ("'; drop table", "Potential SQL injection"),
        ("1=1", "Potential SQL injection"),
        ("union select", "Potential SQL injection"),
    ];

    for (pattern, desc) in &patterns {
        if lower.contains(pattern) {
            return Some(desc.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_input_removes_control_chars() {
        let input = "hello\x00world\x07test\nnewline";
        let result = sanitize_input(input, 100);
        assert_eq!(result, "helloworld\x07test\nnewline".replace('\x07', ""));
        // Actually let's verify properly:
        assert!(!result.contains('\x00'));
        assert!(result.contains('\n'));
    }

    #[test]
    fn sanitize_input_truncates() {
        let result = sanitize_input("hello world", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn sanitize_path_removes_traversal() {
        assert!(!sanitize_path("../../etc/passwd").contains(".."));
        assert!(!sanitize_path("~/.ssh/id_rsa").contains('~'));
    }

    #[test]
    fn sanitize_path_allows_drive_letters() {
        let result = sanitize_path("C:\\Users\\test\\file.txt");
        assert!(result.contains("C:"));
        assert!(result.contains("Users"));
    }

    #[test]
    fn detect_xss() {
        assert!(detect_injection("<script>alert(1)</script>").is_some());
        assert!(detect_injection("javascript:void(0)").is_some());
        assert!(detect_injection("onerror=alert(1)").is_some());
    }

    #[test]
    fn detect_sql_injection() {
        assert!(detect_injection("'; DROP TABLE users").is_some());
        assert!(detect_injection("1 OR 1=1").is_some());
        assert!(detect_injection("UNION SELECT * FROM users").is_some());
    }

    #[test]
    fn no_false_positives() {
        assert!(detect_injection("Hello world").is_none());
        assert!(detect_injection("Get-Process | Select-Object Name").is_none());
    }

    #[test]
    fn sanitize_output_escapes_html() {
        assert_eq!(sanitize_output("<script>"), "&lt;script&gt;");
    }
}
