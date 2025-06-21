use crate::utils::error::Result;
use std::path::{Path, PathBuf};

pub mod archive_branch_iterator;
pub mod branch;
pub mod finish;
pub mod repository;
pub mod validation;
pub mod worktree;

pub use archive_branch_iterator::{ArchiveBranchIterator, HasTimestamp};
pub use branch::{BranchInfo, BranchManager};
pub use finish::{FinishManager, FinishRequest, FinishResult};
pub use repository::GitRepository;
pub use worktree::{WorktreeInfo, WorktreeManager};

pub trait GitOperations {
    fn create_worktree(&self, branch: &str, path: &Path) -> Result<()>;
    fn remove_worktree(&self, path: &Path) -> Result<()>;
    fn finish_session(&self, request: FinishRequest) -> Result<FinishResult>;
    fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>>;
    fn list_branches(&self) -> Result<Vec<BranchInfo>>;
    fn create_branch(&self, name: &str, base: &str) -> Result<()>;
    fn delete_branch(&self, name: &str, force: bool) -> Result<()>;
    fn branch_exists(&self, name: &str) -> Result<bool>;
    fn has_uncommitted_changes(&self) -> Result<bool>;
    fn is_clean_working_tree(&self) -> Result<bool>;
    fn stage_all_changes(&self) -> Result<()>;
    fn archive_branch(&self, branch: &str, prefix: &str) -> Result<String>;
    fn archive_branch_with_session_name(
        &self,
        branch: &str,
        session_name: &str,
        prefix: &str,
    ) -> Result<String>;
    fn restore_archived_branch(&self, archived_branch: &str, prefix: &str) -> Result<String>;
    fn cleanup_stale_worktrees(&self) -> Result<Vec<PathBuf>>;
}

impl GitOperations for GitRepository {
    fn create_worktree(&self, branch: &str, path: &Path) -> Result<()> {
        let manager = WorktreeManager::new(self);
        manager.create_worktree(branch, path)
    }

    fn remove_worktree(&self, path: &Path) -> Result<()> {
        let manager = WorktreeManager::new(self);
        manager.remove_worktree(path)
    }

    fn finish_session(&self, request: FinishRequest) -> Result<FinishResult> {
        let manager = FinishManager::new(self);
        manager.finish_session(request)
    }

    fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let manager = WorktreeManager::new(self);
        manager.list_worktrees()
    }

    fn list_branches(&self) -> Result<Vec<BranchInfo>> {
        let manager = BranchManager::new(self);
        manager.list_branches()
    }

    fn create_branch(&self, name: &str, base: &str) -> Result<()> {
        let manager = BranchManager::new(self);
        manager.create_branch(name, base)
    }

    fn delete_branch(&self, name: &str, force: bool) -> Result<()> {
        let manager = BranchManager::new(self);
        manager.delete_branch(name, force)
    }

    fn branch_exists(&self, name: &str) -> Result<bool> {
        let manager = BranchManager::new(self);
        manager.branch_exists(name)
    }

    fn has_uncommitted_changes(&self) -> Result<bool> {
        GitRepository::has_uncommitted_changes(self)
    }

    fn is_clean_working_tree(&self) -> Result<bool> {
        GitRepository::is_clean_working_tree(self)
    }

    fn stage_all_changes(&self) -> Result<()> {
        GitRepository::stage_all_changes(self)
    }

    fn archive_branch(&self, branch: &str, prefix: &str) -> Result<String> {
        let manager = BranchManager::new(self);
        manager.move_to_archive(branch, prefix)
    }

    fn archive_branch_with_session_name(
        &self,
        branch: &str,
        session_name: &str,
        prefix: &str,
    ) -> Result<String> {
        let manager = BranchManager::new(self);
        manager.move_to_archive_with_session_name(branch, session_name, prefix)
    }

    fn restore_archived_branch(&self, archived_branch: &str, prefix: &str) -> Result<String> {
        let manager = BranchManager::new(self);
        manager.restore_from_archive(archived_branch, prefix)
    }

    fn cleanup_stale_worktrees(&self) -> Result<Vec<PathBuf>> {
        let manager = WorktreeManager::new(self);
        manager.cleanup_stale_worktrees()
    }
}

pub struct GitService {
    repo: GitRepository,
}

impl GitService {
    pub fn discover() -> Result<Self> {
        let repo = GitRepository::discover()?;
        repo.validate()?;
        Ok(Self { repo })
    }

    pub fn discover_from(path: &Path) -> Result<Self> {
        let repo = GitRepository::discover_from(path)?;
        repo.validate()?;
        Ok(Self { repo })
    }

    pub fn repository(&self) -> &GitRepository {
        &self.repo
    }

    pub fn worktree_manager(&self) -> WorktreeManager {
        WorktreeManager::new(&self.repo)
    }

    pub fn branch_manager(&self) -> BranchManager {
        BranchManager::new(&self.repo)
    }

    pub fn validate_session_environment(&self, session_path: &Path) -> Result<SessionEnvironment> {
        let worktree_manager = self.worktree_manager();

        let repo_root = match GitRepository::discover_from(session_path) {
            Ok(repo) => repo.root,
            Err(_) => return Ok(SessionEnvironment::Invalid),
        };

        let git_entry = repo_root.join(".git");
        let is_linked_worktree_root = git_entry.is_file();

        if is_linked_worktree_root {
            let branch = worktree_manager.get_worktree_branch(&repo_root)?;
            return Ok(SessionEnvironment::Worktree { branch });
        }

        let is_worktree = worktree_manager.is_worktree_path(session_path);
        let is_main_repo = repo_root == self.repo.root;

        if !is_worktree && !is_main_repo {
            return Ok(SessionEnvironment::Invalid);
        }

        if is_main_repo {
            return Ok(SessionEnvironment::MainRepository);
        }

        let branch = worktree_manager.get_worktree_branch(session_path)?;

        Ok(SessionEnvironment::Worktree { branch })
    }
}

impl GitOperations for GitService {
    fn create_worktree(&self, branch: &str, path: &Path) -> Result<()> {
        self.repo.create_worktree(branch, path)
    }

    fn remove_worktree(&self, path: &Path) -> Result<()> {
        self.repo.remove_worktree(path)
    }

    fn finish_session(&self, request: FinishRequest) -> Result<FinishResult> {
        self.repo.finish_session(request)
    }

    fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        self.repo.list_worktrees()
    }

    fn list_branches(&self) -> Result<Vec<BranchInfo>> {
        self.repo.list_branches()
    }

    fn create_branch(&self, name: &str, base: &str) -> Result<()> {
        self.repo.create_branch(name, base)
    }

    fn delete_branch(&self, name: &str, force: bool) -> Result<()> {
        self.repo.delete_branch(name, force)
    }

    fn branch_exists(&self, name: &str) -> Result<bool> {
        self.repo.branch_exists(name)
    }

    fn has_uncommitted_changes(&self) -> Result<bool> {
        self.repo.has_uncommitted_changes()
    }

    fn is_clean_working_tree(&self) -> Result<bool> {
        self.repo.is_clean_working_tree()
    }

    fn stage_all_changes(&self) -> Result<()> {
        self.repo.stage_all_changes()
    }

    fn archive_branch(&self, branch: &str, prefix: &str) -> Result<String> {
        self.repo.archive_branch(branch, prefix)
    }

    fn archive_branch_with_session_name(
        &self,
        branch: &str,
        session_name: &str,
        prefix: &str,
    ) -> Result<String> {
        self.repo
            .archive_branch_with_session_name(branch, session_name, prefix)
    }

    fn restore_archived_branch(&self, archived_branch: &str, prefix: &str) -> Result<String> {
        self.repo.restore_archived_branch(archived_branch, prefix)
    }

    fn cleanup_stale_worktrees(&self) -> Result<Vec<PathBuf>> {
        self.repo.cleanup_stale_worktrees()
    }
}

#[derive(Debug, Clone)]
pub enum SessionEnvironment {
    MainRepository,
    Worktree { branch: String },
    Invalid,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;

    #[test]
    fn test_git_operations_trait() {
        let (temp_dir, service) = setup_test_repo();

        let current_branch = service
            .repository()
            .get_current_branch()
            .expect("Failed to get current branch");

        service
            .create_branch("test-trait", &current_branch)
            .expect("Failed to create branch via trait");

        assert!(service
            .branch_exists("test-trait")
            .expect("Failed to check branch"));

        let worktree_path = temp_dir.path().join("trait-worktree");
        service
            .create_worktree("test-trait-wt", &worktree_path)
            .expect("Failed to create worktree via trait");

        assert!(worktree_path.exists());

        let worktrees = service.list_worktrees().expect("Failed to list worktrees");
        assert_eq!(worktrees.len(), 2);
    }

    #[test]
    fn test_session_environment_validation() {
        let (temp_dir, service) = setup_test_repo();

        let main_env = service
            .validate_session_environment(&service.repo.root)
            .expect("Failed to validate main repo");
        match main_env {
            SessionEnvironment::MainRepository => {}
            _ => panic!("Expected MainRepository environment"),
        }

        let worktree_path = temp_dir.path().join("env-test");
        service
            .create_worktree("env-branch", &worktree_path)
            .expect("Failed to create worktree");

        let worktree_env = service
            .validate_session_environment(&worktree_path)
            .expect("Failed to validate worktree");
        match worktree_env {
            SessionEnvironment::Worktree { branch } => {
                assert_eq!(branch, "env-branch");
            }
            _ => panic!("Expected Worktree environment"),
        }

        let invalid_path = temp_dir.path().join("nonexistent");
        let invalid_env = service
            .validate_session_environment(&invalid_path)
            .expect("Failed to validate invalid path");
        match invalid_env {
            SessionEnvironment::Invalid => {}
            _ => panic!("Expected Invalid environment"),
        }
    }

    #[test]
    fn test_manager_access() {
        let (_temp_dir, service) = setup_test_repo();

        let _worktree_manager = service.worktree_manager();
        let _branch_manager = service.branch_manager();
        let _repo = service.repository();
    }
}
