use crate::cli::parser::ListArgs;
use crate::utils::Result;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub base_branch: String,
    pub merge_mode: String,
    pub status: SessionStatus,
    pub last_modified: Option<DateTime<Utc>>,
    pub has_uncommitted_changes: Option<bool>,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    Active,
    Dirty,
    Missing,
    Archived,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Active => "active",
            SessionStatus::Dirty => "dirty",
            SessionStatus::Missing => "missing",
            SessionStatus::Archived => "archived",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            SessionStatus::Active => "✓",
            SessionStatus::Dirty => "●",
            SessionStatus::Missing => "✗",
            SessionStatus::Archived => "📦",
        }
    }
}

pub fn display_sessions(sessions: &[SessionInfo], args: &ListArgs) -> Result<()> {
    let result = if args.quiet {
        display_quiet_sessions(sessions)
    } else if args.verbose {
        display_verbose_sessions(sessions)
    } else {
        display_compact_sessions(sessions)
    };

    if !args.quiet && result.is_ok() {
        println!("\nTip: Use 'para monitor' for interactive session management");
    }

    result
}

pub fn display_quiet_sessions(sessions: &[SessionInfo]) -> Result<()> {
    for session in sessions {
        println!("{}", session.session_id);
    }
    Ok(())
}

pub fn display_compact_sessions(sessions: &[SessionInfo]) -> Result<()> {
    println!(
        "{:<2} {:<30} {:<20} {:<15}",
        "St", "Session", "Branch", "Status"
    );
    println!("{}", "-".repeat(70));

    for session in sessions {
        let current_marker = if session.is_current { "*" } else { " " };
        let status_indicator = session.status.symbol();

        println!(
            "{}{} {:<30} {:<20} {:<15}",
            current_marker,
            status_indicator,
            truncate_string(&session.session_id, 30),
            truncate_string(&session.branch, 20),
            session.status.as_str()
        );
    }

    Ok(())
}

pub fn display_verbose_sessions(sessions: &[SessionInfo]) -> Result<()> {
    for (i, session) in sessions.iter().enumerate() {
        if i > 0 {
            println!();
        }

        let current_marker = if session.is_current { " (current)" } else { "" };

        println!("Session: {}{}", session.session_id, current_marker);
        println!(
            "  Status: {} {}",
            session.status.symbol(),
            session.status.as_str()
        );
        println!("  Branch: {}", session.branch);
        println!("  Base Branch: {}", session.base_branch);
        println!("  Merge Mode: {}", session.merge_mode);

        if session.status != SessionStatus::Archived {
            println!("  Worktree: {}", session.worktree_path.display());

            if let Some(has_changes) = session.has_uncommitted_changes {
                println!(
                    "  Uncommitted Changes: {}",
                    if has_changes { "yes" } else { "no" }
                );
            }
        }

        if let Some(modified) = session.last_modified {
            println!(
                "  Last Modified: {}",
                modified.format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
    }

    Ok(())
}

pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}