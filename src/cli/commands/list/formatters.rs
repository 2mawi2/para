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
    pub session_type: SessionType,
    pub container_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    Active,
    Dirty,
    Missing,
    Archived,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionType {
    Worktree,
    Container,
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
        println!(
            "  Type: {}",
            match session.session_type {
                SessionType::Container => "container",
                SessionType::Worktree => "worktree",
            }
        );
        if let Some(container_status) = &session.container_status {
            println!("  Container: {}", container_status);
        }

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

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

pub fn sort_sessions_by_date(sessions: &mut [SessionInfo]) {
    sessions.sort_by(|a, b| {
        b.last_modified
            .unwrap_or(DateTime::<Utc>::MIN_UTC)
            .cmp(&a.last_modified.unwrap_or(DateTime::<Utc>::MIN_UTC))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;

    fn create_test_session_info(
        session_id: &str,
        branch: &str,
        status: SessionStatus,
        is_current: bool,
    ) -> SessionInfo {
        SessionInfo {
            session_id: session_id.to_string(),
            branch: branch.to_string(),
            worktree_path: PathBuf::from(format!("/path/to/{}", session_id)),
            base_branch: "main".to_string(),
            merge_mode: "squash".to_string(),
            status,
            last_modified: None,
            has_uncommitted_changes: Some(false),
            is_current,
            session_type: SessionType::Worktree,
            container_status: None,
        }
    }

    #[test]
    fn test_session_status_display() {
        assert_eq!(SessionStatus::Active.as_str(), "active");
        assert_eq!(SessionStatus::Dirty.as_str(), "dirty");
        assert_eq!(SessionStatus::Missing.as_str(), "missing");
        assert_eq!(SessionStatus::Archived.as_str(), "archived");

        assert_eq!(SessionStatus::Active.symbol(), "✓");
        assert_eq!(SessionStatus::Dirty.symbol(), "●");
        assert_eq!(SessionStatus::Missing.symbol(), "✗");
        assert_eq!(SessionStatus::Archived.symbol(), "📦");
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("exactly_ten", 11), "exactly_ten");
        assert_eq!(truncate_string("this_is_too_long", 10), "this_is...");
        assert_eq!(truncate_string("abc", 3), "abc");
        assert_eq!(truncate_string("abcd", 3), "...");
    }

    #[test]
    fn test_display_compact_sessions() -> Result<()> {
        let sessions = vec![
            create_test_session_info(
                "test-session-1",
                "para/test-branch-1",
                SessionStatus::Active,
                false,
            ),
            {
                let mut info = create_test_session_info(
                    "current-session",
                    "para/current-branch",
                    SessionStatus::Dirty,
                    true,
                );
                info.has_uncommitted_changes = Some(true);
                info.merge_mode = "merge".to_string();
                info
            },
        ];

        // This should not panic
        let result = display_compact_sessions(&sessions);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_display_verbose_sessions() -> Result<()> {
        let sessions = vec![{
            let mut info = create_test_session_info(
                "verbose-test",
                "para/verbose-branch",
                SessionStatus::Active,
                true,
            );
            info.base_branch = "develop".to_string();
            info.last_modified = Some(Utc::now());
            info
        }];

        // This should not panic
        let result = display_verbose_sessions(&sessions);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_sort_sessions_by_date() {
        let now = Utc::now();
        let earlier = now - chrono::Duration::hours(1);
        let later = now + chrono::Duration::hours(1);

        let mut sessions = vec![
            {
                let mut info =
                    create_test_session_info("middle", "para/middle", SessionStatus::Active, false);
                info.last_modified = Some(now);
                info.has_uncommitted_changes = None;
                info
            },
            {
                let mut info = create_test_session_info(
                    "earliest",
                    "para/earliest",
                    SessionStatus::Active,
                    false,
                );
                info.last_modified = Some(earlier);
                info.has_uncommitted_changes = None;
                info
            },
            {
                let mut info =
                    create_test_session_info("latest", "para/latest", SessionStatus::Active, false);
                info.last_modified = Some(later);
                info.has_uncommitted_changes = None;
                info
            },
            {
                let mut info =
                    create_test_session_info("none", "para/none", SessionStatus::Active, false);
                info.last_modified = None;
                info.has_uncommitted_changes = None;
                info
            },
        ];

        sort_sessions_by_date(&mut sessions);

        // Should be sorted by last_modified descending (latest first)
        assert_eq!(sessions[0].session_id, "latest");
        assert_eq!(sessions[1].session_id, "middle");
        assert_eq!(sessions[2].session_id, "earliest");
        assert_eq!(sessions[3].session_id, "none"); // None should be last
    }
}
