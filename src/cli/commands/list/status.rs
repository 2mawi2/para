use crate::core::git::GitService;
use crate::core::session::SessionStatus as UnifiedSessionStatus;
use crate::utils::Result;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

use super::formatting::{SessionInfo, SessionStatus};

pub fn determine_unified_session_status(
    session_state: &crate::core::session::SessionState,
    git_service: &GitService,
) -> Result<SessionStatus> {
    if !session_state.worktree_path.exists() {
        return Ok(SessionStatus::Missing);
    }

    let worktrees = git_service.list_worktrees()?;
    let worktree_exists = worktrees
        .iter()
        .any(|w| w.path == session_state.worktree_path);

    if !worktree_exists {
        return Ok(SessionStatus::Missing);
    }

    if let Some(service) = git_service_for_path(&session_state.worktree_path) {
        if let Ok(is_clean) = service.is_clean_working_tree() {
            if !is_clean {
                return Ok(SessionStatus::Dirty);
            }
        }
    }

    Ok(SessionStatus::Active)
}

pub fn git_service_for_path(path: &Path) -> Option<GitService> {
    GitService::discover_from(path).ok()
}

pub fn determine_uncommitted_changes(worktree_path: &Path) -> Option<bool> {
    if worktree_path.exists() {
        if let Some(service) = git_service_for_path(worktree_path) {
            service.has_uncommitted_changes().ok()
        } else {
            Some(false)
        }
    } else {
        Some(false)
    }
}

pub fn create_session_info_from_state(
    session_state: &crate::core::session::SessionState,
    has_uncommitted_changes: Option<bool>,
) -> SessionInfo {
    SessionInfo {
        session_id: session_state.name.clone(),
        branch: session_state.branch.clone(),
        worktree_path: session_state.worktree_path.clone(),
        base_branch: "main".to_string(),
        merge_mode: "squash".to_string(),
        status: SessionStatus::Archived,
        last_modified: Some(session_state.created_at),
        has_uncommitted_changes,
        is_current: false,
    }
}

pub fn create_session_info_from_branch(session_id: &str, branch_name: &str) -> SessionInfo {
    SessionInfo {
        session_id: session_id.to_string(),
        branch: branch_name.to_string(),
        worktree_path: PathBuf::new(),
        base_branch: "unknown".to_string(),
        merge_mode: "unknown".to_string(),
        status: SessionStatus::Archived,
        last_modified: None,
        has_uncommitted_changes: None,
        is_current: false,
    }
}

pub fn sort_sessions_by_date(sessions: &mut [SessionInfo]) {
    sessions.sort_by(|a, b| {
        b.last_modified
            .unwrap_or(DateTime::<Utc>::MIN_UTC)
            .cmp(&a.last_modified.unwrap_or(DateTime::<Utc>::MIN_UTC))
    });
}

pub fn extract_session_id_from_archived_branch(
    branch_name: &str,
    branch_prefix: &str,
) -> Option<String> {
    let archive_prefix = format!("{}/archived/", branch_prefix);
    if let Some(stripped) = branch_name.strip_prefix(&archive_prefix) {
        if let Some(session_part) = stripped.split('/').next_back() {
            return Some(session_part.to_string());
        }
    }
    None
}