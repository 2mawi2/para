use crate::config::Config;
use crate::core::ide::IdeManager;
use crate::utils::Result;
use std::path::Path;

use super::task_transformation::update_tasks_json_for_resume;

/// Launches IDE for the specified session path
pub fn launch_ide_for_session(config: &Config, path: &Path) -> Result<()> {
    let ide_manager = IdeManager::new(config);

    // For Claude Code in wrapper mode, always use continuation flag when resuming
    if config.ide.name == "claude" && config.ide.wrapper.enabled {
        println!("▶ resuming Claude Code session with conversation continuation...");
        // Update existing tasks.json to include -c flag
        update_tasks_json_for_resume(path)?;
        ide_manager.launch_with_options(path, false, true)
    } else {
        ide_manager.launch(path, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Config, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
    };
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config() -> Config {
        Config {
            ide: IdeConfig {
                name: "echo".into(),
                command: "echo".into(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: false,
                    name: "cursor".into(),
                    command: "echo".into(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: "subtrees/para".into(),
                state_dir: "/tmp/.para_state".into(),
            },
            git: GitConfig {
                branch_prefix: "para".into(),
                auto_stage: true,
                auto_commit: false,
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".into(),
                preserve_on_finish: false,
                auto_cleanup_days: None,
            },
        }
    }

    #[test]
    fn test_launch_ide_for_session_non_claude() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();

        let result = launch_ide_for_session(&config, temp_dir.path());
        // Should succeed with echo command
        assert!(result.is_ok());
    }

    #[test]
    fn test_launch_ide_for_session_claude_wrapper_mode() {
        let temp_dir = TempDir::new().unwrap();
        let vscode_dir = temp_dir.path().join(".vscode");
        fs::create_dir_all(&vscode_dir).unwrap();

        // Create a tasks.json that needs transformation
        let tasks_file = vscode_dir.join("tasks.json");
        let content = r#"{
  "tasks": [{
    "command": "claude"
  }]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let mut config = create_test_config();
        config.ide.name = "claude".into();
        config.ide.wrapper.enabled = true;

        let result = launch_ide_for_session(&config, temp_dir.path());
        assert!(result.is_ok());

        // Verify tasks.json was updated
        let updated_content = fs::read_to_string(&tasks_file).unwrap();
        assert!(updated_content.contains("claude -c"));
    }
}