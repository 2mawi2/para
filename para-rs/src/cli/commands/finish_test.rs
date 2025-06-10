// This is a new, isolated test file for the finish command.

#[cfg(test)]
mod finish_command_tests {
    use crate::cli::parser::FinishArgs;
    use crate::config::{
        Config, ConfigManager, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
    };
    use crate::core::git::GitService;
    use crate::core::session::SessionManager;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;
    use serde_json;
    use std::result::Result;

    fn setup_test_environment() -> (TempDir, Config, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path().to_path_buf();

        // Init git repo
        Command::new("git").arg("init").current_dir(&repo_path).status().expect("git init failed");
        fs::write(repo_path.join("README.md"), "init").unwrap();
        Command::new("git").arg("add").arg(".").current_dir(&repo_path).status().unwrap();
        Command::new("git").arg("commit").arg("-m").arg("Initial").current_dir(&repo_path).status().unwrap();

        let config = Config {
            ide: IdeConfig {
                name: "test-ide".to_string(),
                command: "test-command".to_string(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: false,
                    name: "".to_string(),
                    command: "".to_string(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: "subtrees".to_string(),
                state_dir: temp_dir.path().join(".para_state").to_string_lossy().to_string(),
            },
            git: GitConfig {
                branch_prefix: "test".to_string(),
                auto_stage: true,
                auto_commit: true,
                default_integration_strategy: crate::cli::parser::IntegrationStrategy::Merge,
            },
            session: SessionConfig {
                preserve_on_finish: true,
                default_name_format: "".to_string(),
                auto_cleanup_days: None,
            },
        };

        (temp_dir, config, repo_path)
    }

    #[test]
    fn test_finish_from_main_repo_with_session_arg() -> crate::utils::Result<()> {
        let (_temp_dir, config, repo_path) = setup_test_environment();
        env::set_current_dir(&repo_path)?;
        let mut session_manager = SessionManager::new(&config);

        let session = session_manager.create_session("test-session".to_string(), None)?;
        fs::write(session.worktree_path.join("file.txt"), "changes")?;

        let args = FinishArgs {
            message: "Finish from root".to_string(),
            session: Some("test-session".to_string()),
            branch: None,
            integrate: false,
        };
        
        // HACK: write config to a predictable path for the test
        let config_path = repo_path.join(".para-test-config.json");
        fs::write(&config_path, serde_json::to_string(&config)?)?;
        env::set_var("PARA_CONFIG_PATH", &config_path);

        let result = crate::cli::commands::finish::execute(args);

        // Clean up env var
        env::remove_var("PARA_CONFIG_PATH");
        
        assert!(result.is_ok(), "Finish failed: {:?}", result.err());
        Ok(())
    }

    #[test]
    fn test_finish_from_within_worktree_auto_detects_session() -> crate::utils::Result<()> {
        let (_temp_dir, config, repo_path) = setup_test_environment();
        env::set_current_dir(&repo_path)?;
        let mut session_manager = SessionManager::new(&config);

        let session = session_manager.create_session("autodetect".to_string(), None)?;
        fs::write(session.worktree_path.join("file.txt"), "changes")?;
        env::set_current_dir(&session.worktree_path)?;

        let args = FinishArgs {
            message: "Finish from worktree".to_string(),
            session: None,
            branch: None,
            integrate: false,
        };

        // HACK: write config to a predictable path for the test
        let config_path = repo_path.join(".para-test-config.json");
        fs::write(&config_path, serde_json::to_string(&config)?)?;
        env::set_var("PARA_CONFIG_PATH", &config_path);

        let result = crate::cli::commands::finish::execute(args);

        // Clean up env var
        env::remove_var("PARA_CONFIG_PATH");

        assert!(result.is_ok(), "Finish failed: {:?}", result.err());
        Ok(())
    }
} 