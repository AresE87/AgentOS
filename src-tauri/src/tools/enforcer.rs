/// Workspace-boundary enforcement for file write/edit operations.
///
/// Prevents tools from writing to system-critical paths and provides
/// awareness of whether writes target the active workspace.

#[derive(Debug, Clone)]
pub enum EnforcementResult {
    Allowed,
    Denied { reason: String },
}

/// System paths that must never be written to by agent tools.
const PROTECTED_SYSTEM_PATHS: &[&str] = &[
    // Windows
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    // Unix
    "/etc",
    "/usr",
    "/sys",
    "/proc",
    "/dev",
    "/boot",
    "/sbin",
    "/lib",
];

/// Check if a file write is allowed given the workspace root.
///
/// - Writes to protected system paths are always denied.
/// - Writes within the workspace are always allowed.
/// - Writes outside the workspace (but not to system paths) are allowed
///   but could be logged as warnings by the caller.
pub fn check_file_write(path: &str, workspace_root: &str) -> EnforcementResult {
    let normalized = normalize_path(path);

    // Always block writes to system-critical directories
    for sp in PROTECTED_SYSTEM_PATHS {
        let norm_sp = normalize_path(sp);
        if normalized.starts_with(&norm_sp) {
            return EnforcementResult::Denied {
                reason: format!("Cannot write to system path: {}", sp),
            };
        }
    }

    // Allow everything else (within or outside workspace)
    // The workspace check is available for callers who want to warn
    let _inside = is_within_workspace(&normalized, &normalize_path(workspace_root));

    EnforcementResult::Allowed
}

/// Normalize a path for consistent comparison: forward slashes, no trailing slash.
fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_string()
}

/// Check whether `path` is inside `root` (or is `root` itself).
fn is_within_workspace(path: &str, root: &str) -> bool {
    let normalized_path = normalize_path(path);
    let normalized_root = format!("{}/", normalize_path(root));
    normalized_path.starts_with(&normalized_root) || normalized_path == normalize_path(root)
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_system_paths() {
        assert!(matches!(
            check_file_write("C:\\Windows\\System32\\evil.dll", "C:\\Users\\me\\project"),
            EnforcementResult::Denied { .. }
        ));
        assert!(matches!(
            check_file_write("/etc/passwd", "/home/user/project"),
            EnforcementResult::Denied { .. }
        ));
        assert!(matches!(
            check_file_write("/usr/bin/malware", "/home/user/project"),
            EnforcementResult::Denied { .. }
        ));
    }

    #[test]
    fn allows_workspace_writes() {
        assert!(matches!(
            check_file_write("C:\\Users\\me\\project\\src\\main.rs", "C:\\Users\\me\\project"),
            EnforcementResult::Allowed
        ));
        assert!(matches!(
            check_file_write("/home/user/project/file.txt", "/home/user/project"),
            EnforcementResult::Allowed
        ));
    }

    #[test]
    fn allows_non_system_outside_workspace() {
        // Writes outside workspace but not to system paths are allowed
        assert!(matches!(
            check_file_write("C:\\Users\\me\\other\\file.txt", "C:\\Users\\me\\project"),
            EnforcementResult::Allowed
        ));
    }

    #[test]
    fn workspace_check_works() {
        assert!(is_within_workspace(
            "/home/user/project/src/main.rs",
            "/home/user/project"
        ));
        assert!(is_within_workspace(
            "C:/Users/me/project/file.txt",
            "C:\\Users\\me\\project"
        ));
        assert!(!is_within_workspace(
            "/home/user/other/file.txt",
            "/home/user/project"
        ));
    }

    // ── H1: Additional targeted tests ────────────────────────────

    #[test]
    fn blocks_windows_system() {
        assert!(matches!(
            check_file_write("C:\\Windows\\test.txt", "C:\\Users\\test"),
            EnforcementResult::Denied { .. }
        ));
    }

    #[test]
    fn allows_workspace_write() {
        assert!(matches!(
            check_file_write("C:/Users/test/project/file.txt", "C:/Users/test/project"),
            EnforcementResult::Allowed
        ));
    }

    #[test]
    fn blocks_etc() {
        assert!(matches!(
            check_file_write("/etc/passwd", "/home/user"),
            EnforcementResult::Denied { .. }
        ));
    }

    #[test]
    fn blocks_program_files() {
        assert!(matches!(
            check_file_write("C:\\Program Files\\app\\config.ini", "C:\\Users\\me\\project"),
            EnforcementResult::Denied { .. }
        ));
    }

    #[test]
    fn blocks_program_files_x86() {
        assert!(matches!(
            check_file_write("C:\\Program Files (x86)\\app\\config.ini", "C:\\Users\\me\\project"),
            EnforcementResult::Denied { .. }
        ));
    }

    #[test]
    fn blocks_unix_boot() {
        assert!(matches!(
            check_file_write("/boot/grub/grub.cfg", "/home/user"),
            EnforcementResult::Denied { .. }
        ));
    }

    #[test]
    fn normalize_handles_mixed_slashes() {
        assert_eq!(normalize_path("C:\\Users\\me/docs"), "C:/Users/me/docs");
        assert_eq!(normalize_path("/home/user/"), "/home/user");
    }
}
