use std::path::{Path, PathBuf};
use crate::error::Result;

pub mod repository;
pub mod worktree;
pub mod branch;
pub mod integration;

pub use repository::GitRepository;
pub use worktree::{WorktreeManager, WorktreeInfo};
pub use branch::{BranchManager, BranchInfo};
pub use integration::{
    IntegrationManager, 
    FinishRequest, 
    FinishResult, 
    IntegrationRequest, 
    IntegrationResult
};

pub trait GitOperations {
    fn create_worktree(&self, branch: &str, path: &Path) -> Result<()>;
    fn remove_worktree(&self, path: &Path) -> Result<()>;
    fn finish_session(&self, request: FinishRequest) -> Result<FinishResult>;
    fn integrate_branch(&self, request: IntegrationRequest) -> Result<IntegrationResult>;
    fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>>;
    fn list_branches(&self) -> Result<Vec<BranchInfo>>;
    fn create_branch(&self, name: &str, base: &str) -> Result<()>;
    fn delete_branch(&self, name: &str, force: bool) -> Result<()>;
    fn branch_exists(&self, name: &str) -> Result<bool>;
    fn get_current_branch(&self) -> Result<String>;
    fn has_uncommitted_changes(&self) -> Result<bool>;
    fn is_clean_working_tree(&self) -> Result<bool>;
    fn stage_all_changes(&self) -> Result<()>;
    fn commit(&self, message: &str) -> Result<()>;
    fn archive_branch(&self, branch: &str, prefix: &str) -> Result<String>;
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
        let manager = IntegrationManager::new(self);
        manager.finish_session(request)
    }

    fn integrate_branch(&self, request: IntegrationRequest) -> Result<IntegrationResult> {
        let manager = IntegrationManager::new(self);
        manager.integrate_branch(request)
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

    fn get_current_branch(&self) -> Result<String> {
        GitRepository::get_current_branch(self)
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

    fn commit(&self, message: &str) -> Result<()> {
        GitRepository::commit(self, message)
    }

    fn archive_branch(&self, branch: &str, prefix: &str) -> Result<String> {
        let manager = BranchManager::new(self);
        manager.move_to_archive(branch, prefix)
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

    pub fn integration_manager(&self) -> IntegrationManager {
        IntegrationManager::new(&self.repo)
    }

    pub fn get_repo_info(&self) -> Result<RepoInfo> {
        Ok(RepoInfo {
            root: self.repo.root.clone(),
            current_branch: self.repo.get_current_branch()?,
            has_uncommitted_changes: self.repo.has_uncommitted_changes()?,
            is_clean: self.repo.is_clean_working_tree()?,
            remote_url: self.repo.get_remote_url()?,
        })
    }

    pub fn validate_session_environment(&self, session_path: &Path) -> Result<SessionEnvironment> {
        let worktree_manager = self.worktree_manager();
        
        let is_worktree = worktree_manager.is_worktree_path(session_path);
        let is_main_repo = session_path == self.repo.root;
        
        if !is_worktree && !is_main_repo {
            return Ok(SessionEnvironment::Invalid);
        }

        if is_main_repo {
            return Ok(SessionEnvironment::MainRepository);
        }

        let branch = worktree_manager.get_worktree_branch(session_path)?;
        let is_clean = worktree_manager.is_worktree_clean(session_path)?;
        
        Ok(SessionEnvironment::Worktree { 
            branch, 
            is_clean 
        })
    }

    pub fn cleanup_all_stale_state(&self) -> Result<CleanupSummary> {
        let worktree_manager = self.worktree_manager();
        let integration_manager = self.integration_manager();
        
        let cleaned_worktrees = worktree_manager.cleanup_stale_worktrees()?;
        integration_manager.cleanup_integration_state()?;
        
        Ok(CleanupSummary {
            cleaned_worktrees,
            cleaned_integration_state: true,
        })
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

    fn integrate_branch(&self, request: IntegrationRequest) -> Result<IntegrationResult> {
        self.repo.integrate_branch(request)
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

    fn get_current_branch(&self) -> Result<String> {
        self.repo.get_current_branch()
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

    fn commit(&self, message: &str) -> Result<()> {
        self.repo.commit(message)
    }

    fn archive_branch(&self, branch: &str, prefix: &str) -> Result<String> {
        self.repo.archive_branch(branch, prefix)
    }

    fn restore_archived_branch(&self, archived_branch: &str, prefix: &str) -> Result<String> {
        self.repo.restore_archived_branch(archived_branch, prefix)
    }

    fn cleanup_stale_worktrees(&self) -> Result<Vec<PathBuf>> {
        self.repo.cleanup_stale_worktrees()
    }
}

#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub root: PathBuf,
    pub current_branch: String,
    pub has_uncommitted_changes: bool,
    pub is_clean: bool,
    pub remote_url: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SessionEnvironment {
    MainRepository,
    Worktree { branch: String, is_clean: bool },
    Invalid,
}

#[derive(Debug)]
pub struct CleanupSummary {
    pub cleaned_worktrees: Vec<PathBuf>,
    pub cleaned_integration_state: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use std::process::Command;

    fn setup_test_repo() -> (TempDir, GitService) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path();

        Command::new("git")
            .current_dir(repo_path)
            .args(&["init"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        fs::write(repo_path.join("README.md"), "# Test Repository").expect("Failed to write README");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["commit", "-m", "Initial commit"])
            .status()
            .expect("Failed to commit README");

        let service = GitService::discover_from(repo_path).expect("Failed to discover repo");
        (temp_dir, service)
    }

    #[test]
    fn test_git_service_discovery() {
        let (_temp_dir, service) = setup_test_repo();
        
        let repo_info = service.get_repo_info().expect("Failed to get repo info");
        assert!(!repo_info.current_branch.is_empty());
        assert!(!repo_info.has_uncommitted_changes);
        assert!(repo_info.is_clean);
    }

    #[test]
    fn test_git_operations_trait() {
        let (temp_dir, service) = setup_test_repo();
        
        let current_branch = service.get_current_branch().expect("Failed to get current branch");
        
        service.create_branch("test-trait", &current_branch)
            .expect("Failed to create branch via trait");
        
        assert!(service.branch_exists("test-trait").expect("Failed to check branch"));
        
        let worktree_path = temp_dir.path().join("trait-worktree");
        service.create_worktree("test-trait-wt", &worktree_path)
            .expect("Failed to create worktree via trait");
        
        assert!(worktree_path.exists());
        
        let worktrees = service.list_worktrees().expect("Failed to list worktrees");
        assert_eq!(worktrees.len(), 2);
    }

    #[test]
    fn test_session_environment_validation() {
        let (temp_dir, service) = setup_test_repo();
        
        let main_env = service.validate_session_environment(&service.repo.root)
            .expect("Failed to validate main repo");
        match main_env {
            SessionEnvironment::MainRepository => {},
            _ => panic!("Expected MainRepository environment"),
        }
        
        let worktree_path = temp_dir.path().join("env-test");
        service.create_worktree("env-branch", &worktree_path)
            .expect("Failed to create worktree");
        
        let worktree_env = service.validate_session_environment(&worktree_path)
            .expect("Failed to validate worktree");
        match worktree_env {
            SessionEnvironment::Worktree { branch, is_clean } => {
                assert_eq!(branch, "env-branch");
                assert!(is_clean);
            },
            _ => panic!("Expected Worktree environment"),
        }
        
        let invalid_path = temp_dir.path().join("nonexistent");
        let invalid_env = service.validate_session_environment(&invalid_path)
            .expect("Failed to validate invalid path");
        match invalid_env {
            SessionEnvironment::Invalid => {},
            _ => panic!("Expected Invalid environment"),
        }
    }

    #[test]
    fn test_manager_access() {
        let (_temp_dir, service) = setup_test_repo();
        
        let _worktree_manager = service.worktree_manager();
        let _branch_manager = service.branch_manager();
        let _integration_manager = service.integration_manager();
        let _repo = service.repository();
    }

    #[test]
    fn test_cleanup_functionality() {
        let (_temp_dir, service) = setup_test_repo();
        
        let summary = service.cleanup_all_stale_state()
            .expect("Failed to cleanup stale state");
        
        assert!(summary.cleaned_integration_state);
    }
}