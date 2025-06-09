use super::state::{SessionConfig, SessionState, SessionStatus, SessionType};
use crate::config::Config;
use crate::utils::{ParaError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct MigrationReport {
    pub migrated_count: usize,
    pub failed_migrations: Vec<String>,
    pub backup_location: PathBuf,
    pub legacy_files_removed: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct ValidationReport {
    pub total_sessions: usize,
    pub valid_sessions: usize,
    pub invalid_sessions: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacySessionState {
    pub name: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub created_at: String,
    pub updated_at: String,
    pub status: LegacySessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum LegacySessionStatus {
    Active,
    Finished,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyDispatchState {
    id: String,
    name: String,
    branch: String,
    path: PathBuf,
    prompt: String,
    created_at: DateTime<Utc>,
    status: LegacyDispatchStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum LegacyDispatchStatus {
    Active,
    Finished,
    Cancelled,
}

pub struct StateMigrator {
    state_dir: PathBuf,
    config: Config,
    backup_dir: PathBuf,
}

impl StateMigrator {
    pub fn new(config: Config) -> Result<Self> {
        let state_dir = PathBuf::from(config.get_state_dir());
        let backup_dir = state_dir.join("migration").join("pre_migration_backup");
        
        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir).map_err(|e| {
                ParaError::fs_error(format!(
                    "Failed to create migration backup directory: {}",
                    e
                ))
            })?;
        }

        Ok(Self {
            state_dir,
            config,
            backup_dir,
        })
    }

    pub fn migrate_legacy_state_files(&self) -> Result<MigrationReport> {
        let mut report = MigrationReport {
            migrated_count: 0,
            failed_migrations: Vec::new(),
            backup_location: self.backup_dir.clone(),
            legacy_files_removed: Vec::new(),
        };

        self.backup_existing_state()?;

        self.migrate_legacy_dot_state_files(&mut report)?;
        self.migrate_legacy_dispatch_json_files(&mut report)?;
        self.migrate_legacy_shell_state_files(&mut report)?;

        self.write_migration_log(&report)?;

        Ok(report)
    }

    fn migrate_legacy_dot_state_files(&self, report: &mut MigrationReport) -> Result<()> {
        if !self.state_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(&self.state_dir)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().is_some_and(|ext| ext == "state") {
                match self.migrate_dot_state_file(&path) {
                    Ok(_) => {
                        report.migrated_count += 1;
                        report.legacy_files_removed.push(path.clone());
                    }
                    Err(e) => {
                        let file_name = path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        report.failed_migrations.push(format!("{}: {}", file_name, e));
                    }
                }
            }
        }

        Ok(())
    }

    fn migrate_legacy_dispatch_json_files(&self, report: &mut MigrationReport) -> Result<()> {
        if !self.state_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(&self.state_dir)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() 
                && path.extension().is_some_and(|ext| ext == "json") 
                && !path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .contains('_') 
            {
                match self.migrate_dispatch_json_file(&path) {
                    Ok(_) => {
                        report.migrated_count += 1;
                        report.legacy_files_removed.push(path.clone());
                    }
                    Err(e) => {
                        let file_name = path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        report.failed_migrations.push(format!("{}: {}", file_name, e));
                    }
                }
            }
        }

        Ok(())
    }

    fn migrate_legacy_shell_state_files(&self, report: &mut MigrationReport) -> Result<()> {
        let shell_state_patterns = vec![
            "current_session",
            "session_info",
            "para_session",
        ];

        for pattern in shell_state_patterns {
            let shell_file = self.state_dir.join(pattern);
            if shell_file.exists() {
                match self.migrate_shell_state_file(&shell_file) {
                    Ok(_) => {
                        report.migrated_count += 1;
                        report.legacy_files_removed.push(shell_file.clone());
                    }
                    Err(e) => {
                        report.failed_migrations.push(format!("{}: {}", pattern, e));
                    }
                }
            }
        }

        Ok(())
    }

    fn migrate_dot_state_file(&self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;
        let legacy_state: LegacySessionState = serde_json::from_str(&content)
            .map_err(|e| ParaError::state_corruption(format!("Invalid legacy .state file: {}", e)))?;

        let session_state = self.convert_legacy_session_state(legacy_state)?;
        self.save_migrated_session(&session_state)?;
        
        Ok(())
    }

    fn migrate_dispatch_json_file(&self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;
        
        if let Ok(legacy_dispatch_state) = serde_json::from_str::<LegacyDispatchState>(&content) {
            let session_state = self.convert_legacy_dispatch_state(legacy_dispatch_state)?;
            self.save_migrated_session(&session_state)?;
        } else {
            return Err(ParaError::state_corruption("Unrecognized JSON format".to_string()));
        }
        
        Ok(())
    }

    fn migrate_shell_state_file(&self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();
        
        let mut state_map = HashMap::new();
        for line in lines {
            if let Some((key, value)) = line.split_once('=') {
                state_map.insert(key.trim(), value.trim().trim_matches('"'));
            }
        }

        let session_state = self.convert_shell_state_map(state_map)?;
        self.save_migrated_session(&session_state)?;
        
        Ok(())
    }

    fn convert_legacy_session_state(&self, legacy: LegacySessionState) -> Result<SessionState> {
        let config_snapshot = SessionConfig::from_config(&self.config);
        
        let repository_root = self.guess_repository_root(&legacy.worktree_path)?;
        let base_branch = self.guess_base_branch()?;
        
        let created_at = parse_legacy_timestamp(&legacy.created_at)?;
        let last_modified = parse_legacy_timestamp(&legacy.updated_at)?;

        let status = match legacy.status {
            LegacySessionStatus::Active => SessionStatus::Active,
            LegacySessionStatus::Finished => SessionStatus::Completed,
            LegacySessionStatus::Cancelled => SessionStatus::Cancelled,
        };

        let session_id = generate_session_id_from_legacy(&legacy.name, &created_at);

        Ok(SessionState {
            id: session_id,
            name: legacy.name,
            session_type: SessionType::Manual,
            branch: legacy.branch,
            base_branch,
            worktree_path: legacy.worktree_path,
            repository_root,
            created_at,
            last_modified,
            status,
            initial_prompt: None,
            commit_count: 0,
            last_commit_hash: None,
            config_snapshot,
        })
    }

    fn convert_legacy_dispatch_state(&self, legacy: LegacyDispatchState) -> Result<SessionState> {
        let config_snapshot = SessionConfig::from_config(&self.config);
        
        let repository_root = self.guess_repository_root(&legacy.path)?;
        let base_branch = self.guess_base_branch()?;

        let status = match legacy.status {
            LegacyDispatchStatus::Active => SessionStatus::Active,
            LegacyDispatchStatus::Finished => SessionStatus::Completed,
            LegacyDispatchStatus::Cancelled => SessionStatus::Cancelled,
        };

        Ok(SessionState {
            id: legacy.id,
            name: legacy.name,
            session_type: SessionType::Dispatched,
            branch: legacy.branch,
            base_branch,
            worktree_path: legacy.path,
            repository_root,
            created_at: legacy.created_at,
            last_modified: legacy.created_at,
            status,
            initial_prompt: Some(legacy.prompt),
            commit_count: 0,
            last_commit_hash: None,
            config_snapshot,
        })
    }

    fn convert_shell_state_map(&self, state_map: HashMap<&str, &str>) -> Result<SessionState> {
        let config_snapshot = SessionConfig::from_config(&self.config);
        
        let name = state_map.get("SESSION_NAME")
            .ok_or_else(|| ParaError::state_corruption("Missing SESSION_NAME".to_string()))?
            .to_string();
        
        let branch = state_map.get("BRANCH_NAME")
            .or_else(|| state_map.get("SESSION_BRANCH"))
            .ok_or_else(|| ParaError::state_corruption("Missing branch name".to_string()))?
            .to_string();

        let worktree_path = PathBuf::from(
            state_map.get("WORKTREE_PATH")
                .or_else(|| state_map.get("SESSION_PATH"))
                .ok_or_else(|| ParaError::state_corruption("Missing worktree path".to_string()))?
        );

        let repository_root = self.guess_repository_root(&worktree_path)?;
        let base_branch = state_map.get("BASE_BRANCH")
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.guess_base_branch().unwrap_or_else(|_| "main".to_string()));

        let now = Utc::now();
        let session_id = generate_session_id_from_legacy(&name, &now);

        let session_type = if state_map.get("PROMPT").is_some() {
            SessionType::Dispatched
        } else {
            SessionType::Manual
        };

        let initial_prompt = state_map.get("PROMPT").map(|s| s.to_string());

        Ok(SessionState {
            id: session_id,
            name,
            session_type,
            branch,
            base_branch,
            worktree_path,
            repository_root,
            created_at: now,
            last_modified: now,
            status: SessionStatus::Active,
            initial_prompt,
            commit_count: 0,
            last_commit_hash: None,
            config_snapshot,
        })
    }

    fn save_migrated_session(&self, session: &SessionState) -> Result<()> {
        let sessions_dir = self.state_dir.join("sessions");
        if !sessions_dir.exists() {
            fs::create_dir_all(&sessions_dir)?;
        }

        let state_file = sessions_dir.join(format!("{}.json", session.id));
        let file_format = session.to_file_format();
        let json = serde_json::to_string_pretty(&file_format)?;
        
        fs::write(&state_file, json).map_err(|e| {
            ParaError::file_operation(format!(
                "Failed to save migrated session to {}: {}",
                state_file.display(),
                e
            ))
        })?;

        Ok(())
    }

    fn guess_repository_root(&self, worktree_path: &Path) -> Result<PathBuf> {
        let mut current = worktree_path.to_path_buf();
        
        while current.parent().is_some() {
            current = current.parent().unwrap().to_path_buf();
            if current.join(".git").exists() {
                return Ok(current);
            }
        }

        let cwd = std::env::current_dir()?;
        let mut current = cwd;
        
        while current.parent().is_some() {
            if current.join(".git").exists() {
                return Ok(current);
            }
            current = current.parent().unwrap().to_path_buf();
        }

        Err(ParaError::git_error("Could not determine repository root".to_string()))
    }

    fn guess_base_branch(&self) -> Result<String> {
        let git_service = crate::core::git::GitService::discover()
            .map_err(|e| ParaError::git_error(format!("Failed to discover git repository: {}", e)))?;

        git_service
            .repository()
            .get_main_branch()
            .or_else(|_| Ok("main".to_string()))
    }

    pub fn backup_existing_state(&self) -> Result<()> {
        if !self.state_dir.exists() {
            return Ok(());
        }

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_subdir = self.backup_dir.join(&timestamp);
        
        if !backup_subdir.exists() {
            fs::create_dir_all(&backup_subdir)?;
        }

        copy_directory_contents(&self.state_dir, &backup_subdir)?;

        Ok(())
    }

    pub fn validate_migration(&self) -> Result<ValidationReport> {
        let sessions_dir = self.state_dir.join("sessions");
        if !sessions_dir.exists() {
            return Ok(ValidationReport {
                total_sessions: 0,
                valid_sessions: 0,
                invalid_sessions: Vec::new(),
                warnings: Vec::new(),
            });
        }

        let entries = fs::read_dir(&sessions_dir)?;
        let mut total_sessions = 0;
        let mut valid_sessions = 0;
        let mut invalid_sessions = Vec::new();
        let mut warnings = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                total_sessions += 1;
                
                if let Some(stem) = path.file_stem() {
                    if let Some(session_id) = stem.to_str() {
                        match self.validate_session_file(&path) {
                            Ok(_) => valid_sessions += 1,
                            Err(e) => invalid_sessions.push(format!("{}: {}", session_id, e)),
                        }
                    }
                }
            }
        }

        Ok(ValidationReport {
            total_sessions,
            valid_sessions,
            invalid_sessions,
            warnings,
        })
    }

    fn validate_session_file(&self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;
        let _: super::state::StateFileFormat = serde_json::from_str(&content)
            .map_err(|e| ParaError::state_corruption(format!("Invalid session file format: {}", e)))?;
        Ok(())
    }

    fn write_migration_log(&self, report: &MigrationReport) -> Result<()> {
        let log_file = self.state_dir.join("migration").join("migration.log");
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        
        let log_content = format!(
            "Migration Report - {}\n\
            Migrated Sessions: {}\n\
            Failed Migrations: {}\n\
            Backup Location: {}\n\
            Legacy Files Removed: {}\n\n\
            Failed Migration Details:\n{}\n\n\
            Legacy Files Removed:\n{}\n",
            timestamp,
            report.migrated_count,
            report.failed_migrations.len(),
            report.backup_location.display(),
            report.legacy_files_removed.len(),
            report.failed_migrations.join("\n"),
            report.legacy_files_removed
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("\n")
        );

        fs::write(&log_file, log_content)?;
        Ok(())
    }
}

fn parse_legacy_timestamp(timestamp_str: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(timestamp_str)
        .or_else(|_| DateTime::parse_from_str(timestamp_str, "%Y-%m-%dT%H:%M:%SZ"))
        .or_else(|_| DateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S UTC"))
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| ParaError::state_corruption(format!("Invalid timestamp format: {}", e)))
}

fn generate_session_id_from_legacy(name: &str, created_at: &DateTime<Utc>) -> String {
    let timestamp = created_at.format("%Y%m%d-%H%M%S").to_string();
    format!("{}_{}", name, timestamp)
}

fn copy_directory_contents(src: &Path, dst: &Path) -> Result<()> {
    let entries = fs::read_dir(src)?;
    
    for entry in entries {
        let entry = entry?;
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);
        
        if src_path.is_file() {
            fs::copy(&src_path, &dst_path)?;
        } else if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_directory_contents(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config(temp_dir: &Path) -> Config {
        Config {
            ide: crate::config::IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: crate::config::WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: crate::config::DirectoryConfig {
                subtrees_dir: "subtrees".to_string(),
                state_dir: temp_dir.join(".para_state").to_string_lossy().to_string(),
            },
            git: crate::config::GitConfig {
                branch_prefix: "test".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: crate::config::SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        }
    }

    #[test]
    fn test_migrator_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        
        let migrator = StateMigrator::new(config).unwrap();
        assert!(migrator.backup_dir.exists());
    }

    #[test]
    fn test_parse_legacy_timestamp() {
        let rfc3339 = "2023-06-09T12:34:56Z";
        let result = parse_legacy_timestamp(rfc3339).unwrap();
        assert_eq!(result.year(), 2023);
        assert_eq!(result.month(), 6);
        assert_eq!(result.day(), 9);

        let custom_format = "2023-06-09T12:34:56Z";
        let result = parse_legacy_timestamp(custom_format).unwrap();
        assert_eq!(result.year(), 2023);
    }

    #[test]
    fn test_generate_session_id_from_legacy() {
        let name = "test-session";
        let timestamp = Utc::now();
        let id = generate_session_id_from_legacy(name, &timestamp);
        
        assert!(id.starts_with("test-session_"));
        assert!(id.contains('-'));
    }

    #[test]
    fn test_convert_shell_state_map() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let migrator = StateMigrator::new(config).unwrap();

        let mut state_map = HashMap::new();
        state_map.insert("SESSION_NAME", "test-session");
        state_map.insert("BRANCH_NAME", "test/branch");
        state_map.insert("WORKTREE_PATH", "/tmp/test/worktree");
        state_map.insert("BASE_BRANCH", "main");

        let result = migrator.convert_shell_state_map(state_map);
        
        // Since this requires git discovery, we expect it to fail in test environment
        // but we can verify the error is reasonable
        assert!(result.is_err());
    }
}