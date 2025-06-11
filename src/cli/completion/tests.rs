use super::*;
use crate::cli::parser::IntegrationStrategy;
use crate::config::{Config, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig};
use tempfile::TempDir;

fn create_test_config(temp_dir: &std::path::Path) -> Config {
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
            subtrees_dir: "subtrees".to_string(),
            state_dir: temp_dir.join(".para_state").to_string_lossy().to_string(),
        },
        git: GitConfig {
            branch_prefix: "pc".to_string(),
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

#[cfg(test)]
mod context_tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_context(command_line: Vec<String>, position: usize) -> CompletionContext {
        let current_word = command_line.get(position).cloned().unwrap_or_default();
        let previous_word = if position > 0 {
            command_line.get(position - 1).cloned()
        } else {
            None
        };

        CompletionContext {
            command_line,
            current_word,
            previous_word,
            position,
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            is_git_repository: false,
        }
    }

    #[test]
    fn test_completion_context_creation() {
        let command_line = vec![
            "para".to_string(),
            "start".to_string(),
            "my-session".to_string(),
        ];
        let context = create_test_context(command_line, 2);

        assert_eq!(context.current_word, "my-session");
        assert_eq!(context.previous_word, Some("start".to_string()));
        assert_eq!(context.position, 2);
        assert_eq!(context.get_subcommand(), Some("start"));
    }

    #[test]
    fn test_flag_completion_detection() {
        let command_line = vec![
            "para".to_string(),
            "start".to_string(),
            "--branch".to_string(),
        ];
        let context = create_test_context(command_line, 2);

        assert!(context.is_completing_flag());
        assert!(!context.is_completing_value_for_flag("--branch"));

        let command_line2 = vec![
            "para".to_string(),
            "start".to_string(),
            "--branch".to_string(),
            "feature".to_string(),
        ];
        let context2 = create_test_context(command_line2, 3);
        assert!(context2.is_completing_value_for_flag("--branch"));
    }

    #[test]
    fn test_file_completion_detection() {
        let command_line = vec![
            "para".to_string(),
            "dispatch".to_string(),
            "--file".to_string(),
            "prompt.txt".to_string(),
        ];
        let context = create_test_context(command_line, 3);

        assert!(context.is_completing_file());
        assert!(context.is_completing_value_for_flag("--file"));
        assert!(context.is_completing_value_for_flag("-f"));
    }

    #[test]
    fn test_session_completion_detection() {
        let command_line = vec![
            "para".to_string(),
            "resume".to_string(),
            "session".to_string(),
        ];
        let context = create_test_context(command_line, 2);

        assert!(context.is_completing_session());
        assert!(!context.should_complete_archived_sessions());

        let recover_command = vec![
            "para".to_string(),
            "recover".to_string(),
            "session".to_string(),
        ];
        let recover_context = create_test_context(recover_command, 2);

        assert!(recover_context.is_completing_session());
        assert!(recover_context.should_complete_archived_sessions());
    }

    #[test]
    fn test_subcommand_detection() {
        let command_line = vec![
            "para".to_string(),
            "finish".to_string(),
            "message".to_string(),
        ];
        let context = create_test_context(command_line, 2);

        assert_eq!(context.get_subcommand(), Some("finish"));
    }
}

#[cfg(test)]
mod dynamic_completion_tests {
    use super::*;

    #[test]
    fn test_subcommand_completions() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let completion = dynamic::DynamicCompletion::new(config);

        let suggestions = completion.get_subcommand_completions();
        assert!(!suggestions.is_empty());

        let start_suggestion = suggestions.iter().find(|s| s.text == "start");
        assert!(start_suggestion.is_some());
        assert!(start_suggestion.unwrap().description.is_some());

        let finish_suggestion = suggestions.iter().find(|s| s.text == "finish");
        assert!(finish_suggestion.is_some());
    }

    #[test]
    fn test_flag_completions() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let completion = dynamic::DynamicCompletion::new(config);

        let dispatch_flags = completion.get_flag_completions(Some("dispatch"));
        assert!(!dispatch_flags.is_empty());
        assert!(dispatch_flags.iter().any(|s| s.text == "--file"));
        assert!(dispatch_flags.iter().any(|s| s.text == "-f"));

        let finish_flags = completion.get_flag_completions(Some("finish"));
        assert!(finish_flags.iter().any(|s| s.text == "--branch"));
        assert!(finish_flags.iter().any(|s| s.text == "--integrate"));

        let list_flags = completion.get_flag_completions(Some("list"));
        assert!(list_flags.iter().any(|s| s.text == "--verbose"));
        assert!(list_flags.iter().any(|s| s.text == "--archived"));
    }

    #[test]
    fn test_completion_filtering() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let completion = dynamic::DynamicCompletion::new(config);

        let mut suggestions = vec![
            CompletionSuggestion::new("start".to_string(), CompletionType::Subcommand),
            CompletionSuggestion::new("finish".to_string(), CompletionType::Subcommand),
            CompletionSuggestion::new("status".to_string(), CompletionType::Subcommand),
        ];

        completion.filter_and_sort(&mut suggestions, "st");
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].text, "start");
        assert_eq!(suggestions[1].text, "status");
    }

    #[test]
    fn test_file_completions() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let completion = dynamic::DynamicCompletion::new(config);

        // Create a test file
        std::fs::write(temp_dir.path().join("test.txt"), "content").unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let context = CompletionContext {
            command_line: vec![
                "para".to_string(),
                "dispatch".to_string(),
                "--file".to_string(),
            ],
            current_word: "".to_string(),
            previous_word: Some("--file".to_string()),
            position: 3,
            working_directory: temp_dir.path().to_path_buf(),
            is_git_repository: false,
        };

        let file_suggestions = completion.get_file_completions(&context);
        assert!(!file_suggestions.is_empty());
        assert!(file_suggestions.iter().any(|s| s.text == "test.txt"));
        assert!(file_suggestions.iter().any(|s| s.text == "subdir/"));
    }
}

#[cfg(test)]
mod generator_tests {
    use super::*;
    use crate::cli::parser::Shell;

    #[test]
    fn test_basic_completion_generation() {
        let bash_completion =
            generators::ShellCompletionGenerator::generate_basic_completion(Shell::Bash);
        assert!(bash_completion.is_ok());
        let bash_script = bash_completion.unwrap();
        assert!(bash_script.contains("para"));
        assert!(bash_script.contains("complete"));

        let zsh_completion =
            generators::ShellCompletionGenerator::generate_basic_completion(Shell::Zsh);
        assert!(zsh_completion.is_ok());
        let zsh_script = zsh_completion.unwrap();
        assert!(zsh_script.contains("para"));
        assert!(zsh_script.contains("compdef"));

        let fish_completion =
            generators::ShellCompletionGenerator::generate_basic_completion(Shell::Fish);
        assert!(fish_completion.is_ok());
        let fish_script = fish_completion.unwrap();
        assert!(fish_script.contains("para"));
        assert!(fish_script.contains("complete"));
    }

    #[test]
    fn test_enhanced_completion_generation() {
        let enhanced_bash =
            generators::ShellCompletionGenerator::generate_enhanced_completion(Shell::Bash);
        assert!(enhanced_bash.is_ok());
        let bash_script = enhanced_bash.unwrap();
        assert!(bash_script.contains("para"));
        assert!(bash_script.contains("_para_dynamic_complete"));
        assert!(bash_script.contains("complete-command"));

        let enhanced_zsh =
            generators::ShellCompletionGenerator::generate_enhanced_completion(Shell::Zsh);
        assert!(enhanced_zsh.is_ok());
        let zsh_script = enhanced_zsh.unwrap();
        assert!(zsh_script.contains("para"));
        assert!(zsh_script.contains("_para"));
    }

    #[test]
    fn test_installation_instructions() {
        let bash_instructions =
            generators::ShellCompletionGenerator::get_installation_instructions(Shell::Bash);
        assert!(bash_instructions.contains("Installation instructions"));
        assert!(bash_instructions.contains("bash"));
        assert!(bash_instructions.contains("~/.bashrc"));

        let zsh_instructions =
            generators::ShellCompletionGenerator::get_installation_instructions(Shell::Zsh);
        assert!(zsh_instructions.contains("Installation instructions"));
        assert!(zsh_instructions.contains("zsh"));
        assert!(zsh_instructions.contains("~/.zshrc"));

        let fish_instructions =
            generators::ShellCompletionGenerator::get_installation_instructions(Shell::Fish);
        assert!(fish_instructions.contains("Installation instructions"));
        assert!(fish_instructions.contains("fish"));
        assert!(fish_instructions.contains("completions"));
    }
}
