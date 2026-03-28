/// Mesh network security
///
/// Security principles:
/// - All connections use TLS
/// - Mesh auth via shared secret (set during pairing)
/// - API keys NEVER leave the local machine
/// - Only task descriptions and results are transmitted
/// - E2E encryption for task payloads (AES-256-GCM)

/// Generate a mesh pairing code (6 alphanumeric characters)
pub fn generate_pairing_code() -> String {
    use std::fmt::Write;
    let bytes: [u8; 3] = rand_bytes();
    let mut code = String::new();
    for b in bytes {
        write!(code, "{:02X}", b).ok();
    }
    code
}

fn rand_bytes() -> [u8; 3] {
    let mut buf = [0u8; 3];
    // Simple PRNG from timestamp for now
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    buf[0] = (ts & 0xFF) as u8;
    buf[1] = ((ts >> 8) & 0xFF) as u8;
    buf[2] = ((ts >> 16) & 0xFF) as u8;
    buf
}

/// Validate a pairing code format
pub fn validate_pairing_code(code: &str) -> bool {
    code.len() == 6 && code.chars().all(|c| c.is_ascii_hexdigit())
}
