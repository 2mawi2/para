use crate::core::git::{GitOperations, GitService};
use crate::core::session::state::SessionState;
use crate::core::session::SessionManager;
use crate::utils::{ParaError, Result};

/// Handle worktree path repairs and recovery
pub fn repair_worktree_path(
    session_state: &mut SessionState,
    git_service: &GitService,
    session_manager: &SessionManager,
    session_name: &str,
) -> Result<()> {
    if !session_state.worktree_path.exists() {
        let branch_to_match = session_state.branch.clone();
        if let Some(wt) = git_service
            .list_worktrees()?
            .into_iter()
            .find(|w| w.branch == branch_to_match)
        {
            session_state.worktree_path = wt.path.clone();
            session_manager.save_state(session_state)?;
        } else if let Some(wt) = git_service.list_worktrees()?.into_iter().find(|w| {
            w.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(session_name))
                .unwrap_or(false)
        }) {
            session_state.worktree_path = wt.path.clone();
            session_manager.save_state(session_state)?;
        } else {
            return Err(ParaError::session_not_found(format!(
                "Session '{}' exists but worktree path '{}' not found",
                session_name,
                session_state.worktree_path.display()
            )));
        }
    }
    Ok(())
}
