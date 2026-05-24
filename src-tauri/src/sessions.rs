use serde::{Deserialize, Serialize};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

const CLAUDE_PROCESS_NAME: &str = "claude";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub pid: u32,
    pub cwd: Option<String>,
    pub context_used: Option<u64>,
    pub context_total: Option<u64>,
}

pub fn scan() -> Vec<Session> {
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    sys.refresh_processes();

    let mut sessions: Vec<Session> = sys
        .processes()
        .iter()
        .filter(|(_, p)| p.name() == CLAUDE_PROCESS_NAME)
        .map(|(pid, _)| {
            let pid_u32 = pid.as_u32();
            Session {
                pid: pid_u32,
                cwd: read_cwd(pid_u32),
                // Filled in by Phase 5 (transcripts.rs).
                context_used: None,
                context_total: None,
            }
        })
        .collect();

    sessions.sort_by_key(|s| s.pid);
    sessions
}

/// Resolve the cwd of a Linux process via /proc/<pid>/cwd. Returns None if
/// the symlink cannot be read (process is gone, permission denied across
/// user namespaces, etc.) — in that case the session is still listed, just
/// without a project path.
#[cfg(target_os = "linux")]
fn read_cwd(pid: u32) -> Option<String> {
    let link = format!("/proc/{pid}/cwd");
    std::fs::read_link(link)
        .ok()
        .and_then(|p| p.to_str().map(String::from))
}

#[cfg(not(target_os = "linux"))]
fn read_cwd(_pid: u32) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_does_not_panic_and_returns_known_shape() {
        let sessions = scan();
        for s in &sessions {
            // pid is real
            assert!(s.pid > 0);
            // cwd, when present, is an absolute path
            if let Some(cwd) = &s.cwd {
                assert!(cwd.starts_with('/'), "cwd should be absolute, got {cwd}");
            }
            // Phase 4 leaves context fields unfilled
            assert!(s.context_used.is_none());
            assert!(s.context_total.is_none());
        }
    }

    #[test]
    fn scan_finds_self_if_named_claude() {
        // Smoke test: when running under `claude` (as we are during the
        // statusline hook session), scan() should find at least one process.
        // We deliberately do not assert > 0 because tests may run from any
        // shell, including not-claude.
        let sessions = scan();
        // Sorted ascending
        let mut prev = 0u32;
        for s in &sessions {
            assert!(s.pid >= prev);
            prev = s.pid;
        }
    }
}
