use super::repository::{execute_git_command, execute_git_command_with_status, GitRepository};
use super::validation::GitValidator;
use crate::utils::error::{ParaError, Result};

/// Trait for types that have a timestamp field for sorting archived branches
pub trait HasTimestamp {
    fn get_timestamp(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
}

pub struct BranchManager<'a> {
    repo: &'a GitRepository,
}

impl<'a> BranchManager<'a> {
    pub fn new(repo: &'a GitRepository) -> Self {
        Self { repo }
    }

    pub fn create_branch(&self, name: &str, base: &str) -> Result<()> {
        self.validate_branch_name(name)?;

        execute_git_command_with_status(self.repo, &["checkout", "-b", name, base])
    }

    pub fn delete_branch(&self, name: &str, force: bool) -> Result<()> {
        self.validate_branch_name(name)?;

        let current_branch = self.repo.get_current_branch()?;
        if current_branch == name {
            return Err(ParaError::git_operation(
                "Cannot delete current branch".to_string(),
            ));
        }

        let args = if force {
            vec!["branch", "-D", name]
        } else {
            vec!["branch", "-d", name]
        };

        execute_git_command_with_status(self.repo, &args)
    }

    pub fn branch_exists(&self, name: &str) -> Result<bool> {
        let result = execute_git_command(
            self.repo,
            &["rev-parse", "--verify", &format!("refs/heads/{}", name)],
        );
        Ok(result.is_ok())
    }

    pub fn list_branches(&self) -> Result<Vec<BranchInfo>> {
        let output = execute_git_command(self.repo, &["branch", "-v"])?;

        let mut branches = Vec::new();
        for line in output.lines() {
            if let Some(branch_info) = self.parse_branch_line(line)? {
                branches.push(branch_info);
            }
        }

        Ok(branches)
    }

    pub fn move_to_archive(&self, branch: &str, prefix: &str) -> Result<String> {
        self.validate_branch_name(branch)?;

        if !self.branch_exists(branch)? {
            return Err(ParaError::git_operation(format!(
                "Branch '{}' does not exist",
                branch
            )));
        }

        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let archived_name = format!("{}/archived/{}/{}", prefix, timestamp, branch);

        execute_git_command_with_status(self.repo, &["branch", "-m", branch, &archived_name])?;

        Ok(archived_name)
    }

    pub fn move_to_archive_with_session_name(
        &self,
        branch: &str,
        session_name: &str,
        prefix: &str,
    ) -> Result<String> {
        self.validate_branch_name(branch)?;

        if !self.branch_exists(branch)? {
            return Err(ParaError::git_operation(format!(
                "Branch '{}' does not exist",
                branch
            )));
        }

        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let archived_name = format!("{}/archived/{}/{}", prefix, timestamp, session_name);

        execute_git_command_with_status(self.repo, &["branch", "-m", branch, &archived_name])?;

        Ok(archived_name)
    }

    pub fn restore_from_archive(&self, archived_branch: &str, prefix: &str) -> Result<String> {
        self.validate_branch_name(archived_branch)?;

        if !self.branch_exists(archived_branch)? {
            return Err(ParaError::git_operation(format!(
                "Archived branch '{}' does not exist",
                archived_branch
            )));
        }

        let archive_prefix = format!("{}/archived/", prefix);
        if !archived_branch.starts_with(&archive_prefix) {
            return Err(ParaError::git_operation(format!(
                "Branch '{}' is not an archived branch with prefix '{}'",
                archived_branch, prefix
            )));
        }

        let original_name = archived_branch
            .strip_prefix(&archive_prefix)
            .and_then(|s| s.split('/').next_back())
            .ok_or_else(|| {
                ParaError::git_operation(
                    "Cannot determine original branch name from archive".to_string(),
                )
            })?;

        let restored_name = if self.branch_exists(original_name)? {
            self.generate_unique_branch_name(original_name)?
        } else {
            original_name.to_string()
        };

        execute_git_command_with_status(
            self.repo,
            &["branch", "-m", archived_branch, &restored_name],
        )?;

        Ok(restored_name)
    }

    pub fn list_archived_branches(&self, prefix: &str) -> Result<Vec<String>> {
        let all_branches = self.list_branches()?;
        let archive_prefix = format!("{}/archived/", prefix);

        Ok(all_branches
            .into_iter()
            .filter(|branch| branch.name.starts_with(&archive_prefix))
            .map(|branch| branch.name)
            .collect())
    }

    pub fn validate_branch_name(&self, name: &str) -> Result<()> {
        GitValidator::validate_branch_name(name)
    }

    pub fn generate_unique_branch_name(&self, base_name: &str) -> Result<String> {
        self.validate_branch_name(base_name)?;

        if !self.branch_exists(base_name)? {
            return Ok(base_name.to_string());
        }

        for i in 1..1000 {
            let candidate = format!("{}-{}", base_name, i);
            if !self.branch_exists(&candidate)? {
                return Ok(candidate);
            }
        }

        Err(ParaError::git_operation(
            "Cannot generate unique branch name after 1000 attempts".to_string(),
        ))
    }

    pub fn get_branch_commit(&self, branch: &str) -> Result<String> {
        execute_git_command(self.repo, &["rev-parse", branch])
    }

    /// Common utility for processing archived branches with a custom processor function.
    /// This eliminates code duplication between archive listing and recovery operations.
    pub fn process_archived_branches<T, F>(&self, prefix: &str, processor: F) -> Result<Vec<T>>
    where
        F: Fn(&str) -> Result<Option<T>>,
        T: HasTimestamp,
    {
        let archived_branches = self.list_archived_branches(prefix)?;

        let mut entries = Vec::new();
        for archived_branch in archived_branches {
            if let Some(entry) = processor(&archived_branch)? {
                entries.push(entry);
            }
        }

        entries.sort_by(|a, b| b.get_timestamp().cmp(a.get_timestamp()));
        Ok(entries)
    }

    fn parse_branch_line(&self, line: &str) -> Result<Option<BranchInfo>> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(None);
        }

        let line = if line.starts_with('*') {
            line.strip_prefix("* ").unwrap_or(line)
        } else {
            line.strip_prefix("  ").unwrap_or(line)
        };

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Ok(None);
        }

        let name = parts[0].to_string();

        Ok(Some(BranchInfo { name }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;

    #[test]
    fn test_create_and_delete_branch() {
        let (_temp_dir, git_service) = setup_test_repo();
        let manager = BranchManager::new(git_service.repository());

        let initial_branch = git_service
            .repository()
            .get_current_branch()
            .expect("Failed to get current branch");

        manager
            .create_branch("test-branch", &initial_branch)
            .expect("Failed to create branch");

        assert!(manager
            .branch_exists("test-branch")
            .expect("Failed to check if branch exists"));

        let current_branch = git_service
            .repository()
            .get_current_branch()
            .expect("Failed to get current branch");
        assert_eq!(current_branch, "test-branch");

        git_service
            .repository()
            .checkout_branch(&initial_branch)
            .expect("Failed to checkout initial branch");

        manager
            .delete_branch("test-branch", false)
            .expect("Failed to delete branch");

        assert!(!manager
            .branch_exists("test-branch")
            .expect("Failed to check if branch exists"));
    }

    #[test]
    fn test_archive_and_restore_branch() {
        let (_temp_dir, git_service) = setup_test_repo();
        let manager = BranchManager::new(git_service.repository());

        let initial_branch = git_service
            .repository()
            .get_current_branch()
            .expect("Failed to get current branch");

        manager
            .create_branch("feature-branch", &initial_branch)
            .expect("Failed to create branch");

        git_service
            .repository()
            .checkout_branch(&initial_branch)
            .expect("Failed to checkout initial branch");

        let archived_name = manager
            .move_to_archive("feature-branch", "para")
            .expect("Failed to archive branch");

        assert!(archived_name.starts_with("para/archived/"));
        assert!(archived_name.ends_with("feature-branch"));
        assert!(!manager
            .branch_exists("feature-branch")
            .expect("Failed to check branch"));
        assert!(manager
            .branch_exists(&archived_name)
            .expect("Failed to check archived branch"));

        let restored_name = manager
            .restore_from_archive(&archived_name, "para")
            .expect("Failed to restore branch");

        assert_eq!(restored_name, "feature-branch");
        assert!(manager
            .branch_exists("feature-branch")
            .expect("Failed to check restored branch"));
        assert!(!manager
            .branch_exists(&archived_name)
            .expect("Failed to check archived branch"));
    }

    #[test]
    fn test_list_archived_branches() {
        let (_temp_dir, git_service) = setup_test_repo();
        let manager = BranchManager::new(git_service.repository());

        let initial_branch = git_service
            .repository()
            .get_current_branch()
            .expect("Failed to get current branch");

        manager
            .create_branch("test1", &initial_branch)
            .expect("Failed to create branch");
        manager
            .create_branch("test2", &initial_branch)
            .expect("Failed to create branch");

        git_service
            .repository()
            .checkout_branch(&initial_branch)
            .expect("Failed to checkout initial branch");

        manager
            .move_to_archive("test1", "para")
            .expect("Failed to archive test1");
        manager
            .move_to_archive("test2", "para")
            .expect("Failed to archive test2");

        let archived = manager
            .list_archived_branches("para")
            .expect("Failed to list archived branches");
        assert_eq!(archived.len(), 2);
        assert!(archived
            .iter()
            .all(|name| name.starts_with("para/archived/")));
        assert!(archived.iter().any(|name| name.ends_with("test1")));
        assert!(archived.iter().any(|name| name.ends_with("test2")));
    }

    #[test]
    fn test_validate_branch_name() {
        let (_temp_dir, git_service) = setup_test_repo();
        let manager = BranchManager::new(git_service.repository());

        assert!(manager.validate_branch_name("valid-branch").is_ok());
        assert!(manager.validate_branch_name("feature/test").is_ok());
        assert!(manager.validate_branch_name("v1.0.0").is_ok());

        let invalid_names = vec![
            "",
            "branch..name",
            "-invalid",
            "invalid/",
            "branch name",
            "@",
            "branch@{",
            "branch~1",
            "refs/heads/test",
        ];

        for invalid_name in invalid_names {
            assert!(
                manager.validate_branch_name(invalid_name).is_err(),
                "Should reject invalid branch name: {}",
                invalid_name
            );
        }
    }

    #[test]
    fn test_generate_unique_branch_name() {
        let (_temp_dir, git_service) = setup_test_repo();
        let manager = BranchManager::new(git_service.repository());

        let initial_branch = git_service
            .repository()
            .get_current_branch()
            .expect("Failed to get current branch");

        let unique_name = manager
            .generate_unique_branch_name("new-feature")
            .expect("Failed to generate unique name");
        assert_eq!(unique_name, "new-feature");

        manager
            .create_branch("existing-branch", &initial_branch)
            .expect("Failed to create branch");

        let unique_name = manager
            .generate_unique_branch_name("existing-branch")
            .expect("Failed to generate unique name");
        assert_eq!(unique_name, "existing-branch-1");
    }
}
