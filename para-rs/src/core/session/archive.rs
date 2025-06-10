use crate::cli::parser::IntegrationStrategy;
use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::{ParaError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub session_name: String,
    pub branch_name: String,
    pub archived_branch: String,
    pub archived_at: String,
    pub commit_hash: String,
    pub commit_message: String,
    pub file_count: usize,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    pub version: String,
    pub created_at: String,
    pub entries: Vec<ArchiveEntry>,
}

#[derive(Debug)]
pub struct ArchiveStats {
    pub total_archives: usize,
    pub total_size_bytes: u64,
    pub oldest_archive: Option<String>,
    pub newest_archive: Option<String>,
    pub by_month: HashMap<String, usize>,
}

pub struct ArchiveManager<'a> {
    config: &'a Config,
    git_service: &'a GitService,
    archive_dir: PathBuf,
}

impl<'a> ArchiveManager<'a> {
    pub fn new(config: &'a Config, git_service: &'a GitService) -> Self {
        let archive_dir = PathBuf::from(config.get_state_dir()).join("archives");
        Self {
            config,
            git_service,
            archive_dir,
        }
    }

    pub fn list_archives(&self) -> Result<Vec<ArchiveEntry>> {
        let archived_branches = self
            .git_service
            .branch_manager()
            .list_archived_branches(self.config.get_branch_prefix())?;

        let mut entries = Vec::new();

        for archived_branch in archived_branches {
            if let Some(entry) = self.create_archive_entry(&archived_branch)? {
                entries.push(entry);
            }
        }

        entries.sort_by(|a, b| b.archived_at.cmp(&a.archived_at));
        Ok(entries)
    }

    pub fn get_archive_stats(&self) -> Result<ArchiveStats> {
        let archives = self.list_archives()?;
        let mut by_month = HashMap::new();
        let mut total_size = 0;

        let mut oldest = None;
        let mut newest = None;

        for archive in &archives {
            total_size += archive.size_bytes;

            if let Ok(dt) = DateTime::parse_from_rfc3339(&archive.archived_at) {
                let month_key = dt.format("%Y-%m").to_string();
                *by_month.entry(month_key).or_insert(0) += 1;

                let timestamp = &archive.archived_at;
                if oldest.is_none() || oldest.as_ref() > Some(timestamp) {
                    oldest = Some(timestamp.clone());
                }
                if newest.is_none() || newest.as_ref() < Some(timestamp) {
                    newest = Some(timestamp.clone());
                }
            }
        }

        Ok(ArchiveStats {
            total_archives: archives.len(),
            total_size_bytes: total_size,
            oldest_archive: oldest,
            newest_archive: newest,
            by_month,
        })
    }

    pub fn export_archive(&self, session_name: &str, export_path: &Path) -> Result<()> {
        let archives = self.list_archives()?;
        let archive = archives
            .iter()
            .find(|a| a.session_name == session_name)
            .ok_or_else(|| ParaError::session_not_found(session_name.to_string()))?;

        let metadata = ArchiveMetadata {
            version: "1.0".to_string(),
            created_at: Utc::now().to_rfc3339(),
            entries: vec![archive.clone()],
        };

        if let Some(parent) = export_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(export_path, json)?;

        Ok(())
    }

    pub fn import_archive(&self, import_path: &Path) -> Result<Vec<String>> {
        if !import_path.exists() {
            return Err(ParaError::file_not_found(
                import_path.to_string_lossy().to_string(),
            ));
        }

        let content = std::fs::read_to_string(import_path)?;
        let metadata: ArchiveMetadata = serde_json::from_str(&content)?;

        let mut imported_sessions = Vec::new();

        for entry in metadata.entries {
            if self.can_import_archive(&entry)? {
                self.import_archive_entry(&entry)?;
                imported_sessions.push(entry.session_name);
            }
        }

        Ok(imported_sessions)
    }

    pub fn cleanup_old_archives(&self, max_age_days: u32) -> Result<usize> {
        let archives = self.list_archives()?;
        let cutoff_date = Utc::now() - chrono::Duration::days(max_age_days as i64);
        let mut cleaned_count = 0;

        for archive in archives {
            if let Ok(archived_date) = DateTime::parse_from_rfc3339(&archive.archived_at) {
                if archived_date.with_timezone(&Utc) < cutoff_date
                    && self.delete_archive(&archive.session_name).is_ok()
                {
                    cleaned_count += 1;
                }
            }
        }

        Ok(cleaned_count)
    }

    pub fn delete_archive(&self, session_name: &str) -> Result<()> {
        let archives = self.list_archives()?;
        let archive = archives
            .iter()
            .find(|a| a.session_name == session_name)
            .ok_or_else(|| ParaError::session_not_found(session_name.to_string()))?;

        let branch_manager = self.git_service.branch_manager();
        branch_manager.force_delete_branch(&archive.archived_branch)?;

        Ok(())
    }

    pub fn find_archive(&self, session_name: &str) -> Result<Option<ArchiveEntry>> {
        let archives = self.list_archives()?;
        Ok(archives
            .into_iter()
            .find(|a| a.session_name == session_name))
    }

    pub fn validate_archive_integrity(&self) -> Result<Vec<String>> {
        let archives = self.list_archives()?;
        let mut issues = Vec::new();

        for archive in archives {
            if let Err(e) = self.validate_single_archive(&archive) {
                issues.push(format!(
                    "Archive '{}' has issues: {}",
                    archive.session_name, e
                ));
            }
        }

        Ok(issues)
    }

    pub fn get_archive_size(&self, session_name: &str) -> Result<u64> {
        let archive = self
            .find_archive(session_name)?
            .ok_or_else(|| ParaError::session_not_found(session_name.to_string()))?;

        Ok(archive.size_bytes)
    }

    fn create_archive_entry(&self, archived_branch: &str) -> Result<Option<ArchiveEntry>> {
        let archive_prefix = format!("{}/archived/", self.config.get_branch_prefix());

        if !archived_branch.starts_with(&archive_prefix) {
            return Ok(None);
        }

        let suffix = archived_branch.strip_prefix(&archive_prefix).unwrap();
        let parts: Vec<&str> = suffix.split('/').collect();

        if parts.len() != 2 {
            return Ok(None);
        }

        let timestamp_str = parts[0];
        let session_name = parts[1];

        let archived_at = self.parse_timestamp_to_rfc3339(timestamp_str);

        let branch_manager = self.git_service.branch_manager();
        let commit_hash = branch_manager.get_branch_commit(archived_branch)?;

        let commit_message = self.get_commit_message(&commit_hash).unwrap_or_default();

        let (file_count, size_bytes) = self.estimate_archive_size(&commit_hash)?;

        Ok(Some(ArchiveEntry {
            session_name: session_name.to_string(),
            branch_name: archived_branch.to_string(),
            archived_branch: archived_branch.to_string(),
            archived_at,
            commit_hash,
            commit_message,
            file_count,
            size_bytes,
        }))
    }

    fn parse_timestamp_to_rfc3339(&self, timestamp: &str) -> String {
        if timestamp.len() == 15 {
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(timestamp, "%Y%m%d-%H%M%S") {
                return dt.and_utc().to_rfc3339();
            }
        }

        Utc::now().to_rfc3339()
    }

    fn get_commit_message(&self, commit_hash: &str) -> Result<String> {
        use crate::core::git::repository::execute_git_command;

        let output = execute_git_command(
            self.git_service.repository(),
            &["log", "--format=%s", "-n", "1", commit_hash],
        )?;

        Ok(output.trim().to_string())
    }

    fn estimate_archive_size(&self, commit_hash: &str) -> Result<(usize, u64)> {
        use crate::core::git::repository::execute_git_command;

        let files_output = execute_git_command(
            self.git_service.repository(),
            &["ls-tree", "-r", "--name-only", commit_hash],
        )?;

        let file_count = files_output.lines().count();

        let size_output = execute_git_command(
            self.git_service.repository(),
            &["cat-file", "-s", commit_hash],
        );

        let size_bytes = if let Ok(size_str) = size_output {
            size_str.trim().parse::<u64>().unwrap_or(0)
        } else {
            file_count as u64 * 1024
        };

        Ok((file_count, size_bytes))
    }

    fn can_import_archive(&self, entry: &ArchiveEntry) -> Result<bool> {
        let branch_manager = self.git_service.branch_manager();
        Ok(!branch_manager.branch_exists(&entry.archived_branch)?)
    }

    fn import_archive_entry(&self, _entry: &ArchiveEntry) -> Result<()> {
        Err(ParaError::not_implemented("archive import"))
    }

    fn validate_single_archive(&self, archive: &ArchiveEntry) -> Result<()> {
        let branch_manager = self.git_service.branch_manager();

        if !branch_manager.branch_exists(&archive.archived_branch)? {
            return Err(ParaError::git_operation(
                "Archived branch no longer exists".to_string(),
            ));
        }

        let current_commit = branch_manager.get_branch_commit(&archive.archived_branch)?;
        if current_commit != archive.commit_hash {
            return Err(ParaError::state_corruption(
                "Commit hash mismatch".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig};
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn create_test_config(temp_dir: &Path) -> Config {
        Config {
            ide: IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: temp_dir.join("subtrees").to_string_lossy().to_string(),
                state_dir: temp_dir.join(".para_state").to_string_lossy().to_string(),
            },
            git: GitConfig {
                branch_prefix: "test".to_string(),
                auto_stage: true,
                auto_commit: false,
                default_integration_strategy: IntegrationStrategy::Squash,
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        }
    }

    fn setup_test_repo() -> (TempDir, GitService, Config) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path().join("repo");
        fs::create_dir_all(&repo_path).expect("Failed to create repo dir");

        Command::new("git")
            .current_dir(&repo_path)
            .args(&["init", "--initial-branch=main"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(&repo_path)
            .args(&["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(&repo_path)
            .args(&["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        Command::new("git")
            .current_dir(&repo_path)
            .args(&["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(&repo_path)
            .args(&["commit", "-m", "Initial commit"])
            .status()
            .expect("Failed to commit README");

        let git_service = GitService::discover_from(&repo_path).expect("Failed to discover repo");
        let config = create_test_config(temp_dir.path());

        (temp_dir, git_service, config)
    }

    #[test]
    fn test_archive_manager_creation() {
        let (_temp_dir, git_service, config) = setup_test_repo();
        let _archive_manager = ArchiveManager::new(&config, &git_service);
    }

    #[test]
    fn test_list_empty_archives() {
        let (_temp_dir, git_service, config) = setup_test_repo();
        let archive_manager = ArchiveManager::new(&config, &git_service);

        let archives = archive_manager.list_archives().unwrap();
        assert!(archives.is_empty());
    }

    #[test]
    fn test_archive_stats_empty() {
        let (_temp_dir, git_service, config) = setup_test_repo();
        let archive_manager = ArchiveManager::new(&config, &git_service);

        let stats = archive_manager.get_archive_stats().unwrap();
        assert_eq!(stats.total_archives, 0);
        assert_eq!(stats.total_size_bytes, 0);
        assert!(stats.oldest_archive.is_none());
        assert!(stats.newest_archive.is_none());
    }

    #[test]
    fn test_timestamp_parsing() {
        let (_temp_dir, git_service, config) = setup_test_repo();
        let archive_manager = ArchiveManager::new(&config, &git_service);

        let rfc3339 = archive_manager.parse_timestamp_to_rfc3339("20240301-120000");
        assert!(rfc3339.contains("2024-03-01T12:00:00"));
    }

    #[test]
    fn test_find_nonexistent_archive() {
        let (_temp_dir, git_service, config) = setup_test_repo();
        let archive_manager = ArchiveManager::new(&config, &git_service);

        let result = archive_manager.find_archive("nonexistent");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
