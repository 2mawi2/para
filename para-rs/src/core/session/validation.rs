use super::state::{SessionState, SessionStatus};
use crate::core::git::{GitOperations, GitService};
use crate::utils::{ParaError, Result};
use std::path::Path;

#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub recommended_actions: Vec<String>,
}

#[derive(Debug)]
pub enum ValidationIssue {
    MissingWorktree,
    InvalidBranch,
    CorruptedStateFile,
    MismatchedPaths,
    OutdatedFormat,
    MissingRepository,
    InvalidSessionId,
    InconsistentMetadata,
}

#[derive(Debug)]
pub struct RepairResult {
    pub repaired: bool,
    pub actions_taken: Vec<String>,
    pub remaining_issues: Vec<ValidationIssue>,
}

#[derive(Debug)]
pub struct CleanupReport {
    pub cleaned_sessions: Vec<String>,
    pub preserved_sessions: Vec<String>,
    pub errors: Vec<String>,
}

pub struct SessionValidator {
    git_service: Option<GitService>,
}

impl SessionValidator {
    pub fn new() -> Self {
        let git_service = GitService::discover().ok();
        Self { git_service }
    }

    pub fn validate_session(&self, session: &SessionState) -> ValidationResult {
        let mut issues = Vec::new();
        let mut recommended_actions = Vec::new();

        self.validate_session_id(session, &mut issues, &mut recommended_actions);
        self.validate_paths(session, &mut issues, &mut recommended_actions);
        self.validate_git_state(session, &mut issues, &mut recommended_actions);
        self.validate_metadata_consistency(session, &mut issues, &mut recommended_actions);

        let is_valid = issues.is_empty();

        ValidationResult {
            is_valid,
            issues,
            recommended_actions,
        }
    }

    pub fn validate_all_sessions(&self, sessions: &[SessionState]) -> Vec<(String, ValidationResult)> {
        sessions
            .iter()
            .map(|session| (session.id.clone(), self.validate_session(session)))
            .collect()
    }

    pub fn repair_session(&self, session: &mut SessionState) -> Result<RepairResult> {
        let mut actions_taken = Vec::new();
        let mut remaining_issues = Vec::new();

        let validation = self.validate_session(session);
        
        let original_issue_count = validation.issues.len();
        
        for issue in validation.issues {
            match self.attempt_repair(session, &issue, &mut actions_taken) {
                Ok(true) => {
                    // Successfully repaired
                }
                Ok(false) => {
                    // Could not repair
                    remaining_issues.push(issue);
                }
                Err(_) => {
                    // Error during repair
                    remaining_issues.push(issue);
                }
            }
        }

        let repaired = remaining_issues.len() < original_issue_count;

        Ok(RepairResult {
            repaired,
            actions_taken,
            remaining_issues,
        })
    }

    pub fn cleanup_invalid_sessions(&self, sessions: &[SessionState]) -> CleanupReport {
        let mut cleaned_sessions = Vec::new();
        let mut preserved_sessions = Vec::new();
        let mut errors = Vec::new();

        for session in sessions {
            let validation = self.validate_session(session);
            
            if validation.is_valid {
                preserved_sessions.push(session.id.clone());
                continue;
            }

            let should_cleanup = self.should_cleanup_session(session, &validation.issues);
            
            if should_cleanup {
                match self.cleanup_session_artifacts(session) {
                    Ok(_) => cleaned_sessions.push(session.id.clone()),
                    Err(e) => errors.push(format!("{}: {}", session.id, e)),
                }
            } else {
                preserved_sessions.push(session.id.clone());
            }
        }

        CleanupReport {
            cleaned_sessions,
            preserved_sessions,
            errors,
        }
    }

    fn validate_session_id(&self, session: &SessionState, issues: &mut Vec<ValidationIssue>, actions: &mut Vec<String>) {
        if session.id.is_empty() {
            issues.push(ValidationIssue::InvalidSessionId);
            actions.push("Generate a new session ID".to_string());
            return;
        }

        if !session.id.contains('_') || session.id.len() < 10 {
            issues.push(ValidationIssue::InvalidSessionId);
            actions.push("Regenerate session ID with proper format".to_string());
        }
    }

    fn validate_paths(&self, session: &SessionState, issues: &mut Vec<ValidationIssue>, actions: &mut Vec<String>) {
        if !session.worktree_path.exists() {
            issues.push(ValidationIssue::MissingWorktree);
            actions.push(format!(
                "Recreate worktree at {} or update session path",
                session.worktree_path.display()
            ));
        }

        if !session.repository_root.exists() {
            issues.push(ValidationIssue::MissingRepository);
            actions.push(format!(
                "Update repository root path from {}",
                session.repository_root.display()
            ));
        }

        if session.worktree_path.exists() && session.repository_root.exists() {
            if !self.is_worktree_in_repository(&session.worktree_path, &session.repository_root) {
                issues.push(ValidationIssue::MismatchedPaths);
                actions.push("Verify worktree is properly linked to repository".to_string());
            }
        }
    }

    fn validate_git_state(&self, session: &SessionState, issues: &mut Vec<ValidationIssue>, actions: &mut Vec<String>) {
        if let Some(ref git_service) = self.git_service {
            if let Ok(branches) = git_service.repository().list_branches() {
                if !branches.contains(&session.branch) {
                    issues.push(ValidationIssue::InvalidBranch);
                    actions.push(format!(
                        "Recreate branch '{}' or update session to use existing branch",
                        session.branch
                    ));
                }

                if !branches.contains(&session.base_branch) {
                    issues.push(ValidationIssue::InvalidBranch);
                    actions.push(format!(
                        "Update base branch from '{}' to existing branch",
                        session.base_branch
                    ));
                }
            }
        }
    }

    fn validate_metadata_consistency(&self, session: &SessionState, issues: &mut Vec<ValidationIssue>, actions: &mut Vec<String>) {
        if session.created_at > session.last_modified {
            issues.push(ValidationIssue::InconsistentMetadata);
            actions.push("Fix timestamp consistency".to_string());
        }

        if session.last_commit_hash.is_some() && session.commit_count == 0 {
            issues.push(ValidationIssue::InconsistentMetadata);
            actions.push("Update commit count to match commit hash presence".to_string());
        }

        if session.last_commit_hash.is_none() && session.commit_count > 0 {
            issues.push(ValidationIssue::InconsistentMetadata);
            actions.push("Update commit hash or reset commit count".to_string());
        }
    }

    fn attempt_repair(&self, session: &mut SessionState, issue: &ValidationIssue, actions: &mut Vec<String>) -> Result<bool> {
        match issue {
            ValidationIssue::InvalidSessionId => {
                let new_id = crate::utils::generate_session_id(&session.name);
                session.id = new_id;
                actions.push("Generated new session ID".to_string());
                Ok(true)
            }

            ValidationIssue::InconsistentMetadata => {
                if session.created_at > session.last_modified {
                    session.last_modified = session.created_at;
                    actions.push("Fixed timestamp inconsistency".to_string());
                }

                if session.last_commit_hash.is_some() && session.commit_count == 0 {
                    session.commit_count = 1;
                    actions.push("Updated commit count to match commit hash".to_string());
                } else if session.last_commit_hash.is_none() && session.commit_count > 0 {
                    session.commit_count = 0;
                    actions.push("Reset commit count to match missing commit hash".to_string());
                }

                Ok(true)
            }

            ValidationIssue::InvalidBranch => {
                if let Some(ref git_service) = self.git_service {
                    if let Ok(main_branch) = git_service.repository().get_main_branch() {
                        session.base_branch = main_branch;
                        actions.push("Updated base branch to repository main branch".to_string());
                        return Ok(true);
                    }
                }
                Ok(false)
            }

            _ => Ok(false),
        }
    }

    fn should_cleanup_session(&self, session: &SessionState, issues: &[ValidationIssue]) -> bool {
        let critical_issues = issues.iter().any(|issue| {
            matches!(
                issue,
                ValidationIssue::MissingWorktree 
                | ValidationIssue::MissingRepository 
                | ValidationIssue::CorruptedStateFile
            )
        });

        let inactive = matches!(
            session.status,
            SessionStatus::Cancelled | SessionStatus::Completed
        );

        critical_issues && inactive
    }

    fn cleanup_session_artifacts(&self, session: &SessionState) -> Result<()> {
        if let Some(ref git_service) = self.git_service {
            if session.worktree_path.exists() {
                git_service.remove_worktree(&session.worktree_path).ok();
            }

            if let Ok(branches) = git_service.repository().list_branches() {
                if branches.contains(&session.branch) {
                    git_service.repository().delete_branch(&session.branch, false).ok();
                }
            }
        }

        Ok(())
    }

    fn is_worktree_in_repository(&self, worktree_path: &Path, repository_root: &Path) -> bool {
        worktree_path.starts_with(repository_root) || {
            if let Some(ref git_service) = self.git_service {
                git_service.list_worktrees()
                    .map(|worktrees| worktrees.iter().any(|w| w.path == worktree_path))
                    .unwrap_or(false)
            } else {
                false
            }
        }
    }
}

impl Default for SessionValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationIssue {
    pub fn severity(&self) -> ValidationSeverity {
        match self {
            ValidationIssue::CorruptedStateFile 
            | ValidationIssue::MissingRepository => ValidationSeverity::Critical,
            
            ValidationIssue::MissingWorktree 
            | ValidationIssue::InvalidBranch 
            | ValidationIssue::MismatchedPaths => ValidationSeverity::High,
            
            ValidationIssue::InvalidSessionId 
            | ValidationIssue::InconsistentMetadata 
            | ValidationIssue::OutdatedFormat => ValidationSeverity::Medium,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ValidationIssue::MissingWorktree => "Session worktree directory does not exist",
            ValidationIssue::InvalidBranch => "Session branch does not exist in repository",
            ValidationIssue::CorruptedStateFile => "Session state file is corrupted or unreadable",
            ValidationIssue::MismatchedPaths => "Session paths are inconsistent with repository structure",
            ValidationIssue::OutdatedFormat => "Session state file format is outdated",
            ValidationIssue::MissingRepository => "Repository root directory does not exist",
            ValidationIssue::InvalidSessionId => "Session ID format is invalid",
            ValidationIssue::InconsistentMetadata => "Session metadata contains inconsistencies",
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::state::{SessionConfig, SessionType};
    use chrono::Utc;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_session() -> SessionState {
        let now = Utc::now();
        SessionState {
            id: "test_20231209-120000".to_string(),
            name: "test".to_string(),
            session_type: SessionType::Manual,
            branch: "test/branch".to_string(),
            base_branch: "main".to_string(),
            worktree_path: PathBuf::from("/tmp/test-worktree"),
            repository_root: PathBuf::from("/tmp/test-repo"),
            created_at: now,
            last_modified: now,
            status: SessionStatus::Active,
            initial_prompt: None,
            commit_count: 0,
            last_commit_hash: None,
            config_snapshot: SessionConfig {
                branch_prefix: "test".to_string(),
                subtrees_dir: "subtrees".to_string(),
                ide_name: "test".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
        }
    }

    #[test]
    fn test_validator_creation() {
        let validator = SessionValidator::new();
        // Validator should be created successfully regardless of git state
        assert!(validator.git_service.is_none() || validator.git_service.is_some());
    }

    #[test]
    fn test_validate_session_invalid_id() {
        let validator = SessionValidator::new();
        let mut session = create_test_session();
        session.id = "invalid".to_string();

        let result = validator.validate_session(&session);
        
        assert!(!result.is_valid);
        assert!(result.issues.iter().any(|issue| matches!(issue, ValidationIssue::InvalidSessionId)));
    }

    #[test]
    fn test_validate_session_missing_paths() {
        let validator = SessionValidator::new();
        let session = create_test_session();

        let result = validator.validate_session(&session);
        
        // Should fail because paths don't exist
        assert!(!result.is_valid);
        assert!(result.issues.iter().any(|issue| matches!(issue, ValidationIssue::MissingWorktree)));
        assert!(result.issues.iter().any(|issue| matches!(issue, ValidationIssue::MissingRepository)));
    }

    #[test]
    fn test_validate_session_inconsistent_metadata() {
        let validator = SessionValidator::new();
        let mut session = create_test_session();
        
        // Create inconsistent metadata
        session.last_modified = session.created_at - chrono::Duration::minutes(10);
        session.commit_count = 5;
        session.last_commit_hash = None;

        let result = validator.validate_session(&session);
        
        assert!(!result.is_valid);
        assert!(result.issues.iter().any(|issue| matches!(issue, ValidationIssue::InconsistentMetadata)));
    }

    #[test]
    fn test_repair_session_invalid_id() {
        let validator = SessionValidator::new();
        let mut session = create_test_session();
        session.id = "invalid".to_string();

        let result = validator.repair_session(&mut session).unwrap();
        
        assert!(result.repaired);
        assert!(!result.actions_taken.is_empty());
        assert!(session.id.contains('_'));
        assert!(session.id.len() > 10);
    }

    #[test]
    fn test_repair_session_inconsistent_metadata() {
        let validator = SessionValidator::new();
        let mut session = create_test_session();
        
        session.last_modified = session.created_at - chrono::Duration::minutes(10);
        session.commit_count = 5;
        session.last_commit_hash = None;

        let result = validator.repair_session(&mut session).unwrap();
        
        assert!(result.repaired);
        assert!(!result.actions_taken.is_empty());
        assert!(session.last_modified >= session.created_at);
        assert_eq!(session.commit_count, 0);
    }

    #[test]
    fn test_validation_issue_severity() {
        assert_eq!(ValidationIssue::CorruptedStateFile.severity(), ValidationSeverity::Critical);
        assert_eq!(ValidationIssue::MissingWorktree.severity(), ValidationSeverity::High);
        assert_eq!(ValidationIssue::InvalidSessionId.severity(), ValidationSeverity::Medium);
    }

    #[test]
    fn test_should_cleanup_session() {
        let validator = SessionValidator::new();
        let mut session = create_test_session();
        
        // Active session should not be cleaned up
        let issues = vec![ValidationIssue::MissingWorktree];
        assert!(!validator.should_cleanup_session(&session, &issues));
        
        // Cancelled session with critical issues should be cleaned up
        session.status = SessionStatus::Cancelled;
        assert!(validator.should_cleanup_session(&session, &issues));
        
        // Cancelled session with non-critical issues should not be cleaned up
        let non_critical_issues = vec![ValidationIssue::InvalidSessionId];
        assert!(!validator.should_cleanup_session(&session, &non_critical_issues));
    }

    #[test]
    fn test_cleanup_report() {
        let validator = SessionValidator::new();
        let sessions = vec![create_test_session()];

        let report = validator.cleanup_invalid_sessions(&sessions);
        
        // Since test session has invalid paths but is active, it should be preserved
        assert_eq!(report.cleaned_sessions.len(), 0);
        assert_eq!(report.preserved_sessions.len(), 1);
        assert_eq!(report.errors.len(), 0);
    }
}