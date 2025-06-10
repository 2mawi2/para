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
mod completion_tests {
    use super::*;

    #[test]
    fn test_completion_suggestion_creation() {
        let suggestion = CompletionSuggestion::new("test".to_string(), CompletionType::Command);
        assert_eq!(suggestion.text, "test");
        assert_eq!(suggestion.completion_type, CompletionType::Command);
        assert!(suggestion.description.is_none());

        let with_desc = suggestion.with_description("Test command".to_string());
        assert_eq!(with_desc.description, Some("Test command".to_string()));
    }

    #[test]
    fn test_completion_response_filtering() {
        let suggestions = vec![
            CompletionSuggestion::new("start".to_string(), CompletionType::Subcommand),
            CompletionSuggestion::new("status".to_string(), CompletionType::Subcommand),
            CompletionSuggestion::new("finish".to_string(), CompletionType::Subcommand),
        ];

        let mut response = CompletionResponse::new(suggestions, CompletionType::Subcommand);
        response.filter_by_prefix("st");

        assert_eq!(response.suggestions.len(), 2);
        assert!(response.suggestions.iter().any(|s| s.text == "start"));
        assert!(response.suggestions.iter().any(|s| s.text == "status"));
    }

    #[test]
    fn test_completion_response_sorting() {
        let suggestions = vec![
            CompletionSuggestion::new("zebra".to_string(), CompletionType::Value),
            CompletionSuggestion::new("alpha".to_string(), CompletionType::Value),
            CompletionSuggestion::new("beta".to_string(), CompletionType::Value),
        ];

        let mut response = CompletionResponse::new(suggestions, CompletionType::Value);
        response.sort();

        assert_eq!(response.suggestions[0].text, "alpha");
        assert_eq!(response.suggestions[1].text, "beta");
        assert_eq!(response.suggestions[2].text, "zebra");
    }

    #[test]
    fn test_completion_response_limit() {
        let suggestions = vec![
            CompletionSuggestion::new("one".to_string(), CompletionType::Value),
            CompletionSuggestion::new("two".to_string(), CompletionType::Value),
            CompletionSuggestion::new("three".to_string(), CompletionType::Value),
            CompletionSuggestion::new("four".to_string(), CompletionType::Value),
        ];

        let mut response = CompletionResponse::new(suggestions, CompletionType::Value);
        response.limit(2);

        assert_eq!(response.suggestions.len(), 2);
    }
}

#[cfg(test)]
mod context_tests {
    use super::*;

    #[test]
    fn test_completion_context_creation() {
        let command_line = vec![
            "para".to_string(),
            "start".to_string(),
            "my-session".to_string(),
        ];
        let context = CompletionContext::new(command_line, 2);

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
        let context = CompletionContext::new(command_line, 2);

        assert!(context.is_completing_flag());
        assert!(!context.is_completing_value_for_flag("--branch"));

        let command_line2 = vec![
            "para".to_string(),
            "start".to_string(),
            "--branch".to_string(),
            "feature".to_string(),
        ];
        let context2 = CompletionContext::new(command_line2, 3);
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
        let context = CompletionContext::new(command_line, 3);

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
        let context = CompletionContext::new(command_line, 2);

        assert!(context.is_completing_session());
        assert!(!context.should_complete_archived_sessions());

        let recover_command = vec![
            "para".to_string(),
            "recover".to_string(),
            "session".to_string(),
        ];
        let recover_context = CompletionContext::new(recover_command, 2);

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
        let context = CompletionContext::new(command_line, 2);

        assert_eq!(context.get_subcommand(), Some("finish"));
        assert_eq!(context.get_subcommand_args(), &["message"]);
    }

    #[test]
    fn test_completion_type_detection() {
        let flag_context = CompletionContext::new(
            vec![
                "para".to_string(),
                "start".to_string(),
                "--branch".to_string(),
            ],
            2,
        );
        assert_eq!(
            flag_context.get_completion_type(),
            context::CompletionType::Flag
        );

        let subcommand_context =
            CompletionContext::new(vec!["para".to_string(), "sta".to_string()], 1);
        assert_eq!(
            subcommand_context.get_completion_type(),
            context::CompletionType::Subcommand
        );

        let session_context = CompletionContext::new(
            vec!["para".to_string(), "resume".to_string(), "sess".to_string()],
            2,
        );
        assert_eq!(
            session_context.get_completion_type(),
            context::CompletionType::Session
        );
    }

    #[test]
    fn test_git_repository_requirements() {
        let context = CompletionContext::new(vec!["para".to_string(), "start".to_string()], 1);
        assert!(context.needs_git_repository());

        let config_context =
            CompletionContext::new(vec!["para".to_string(), "config".to_string()], 1);
        assert!(!config_context.needs_git_repository());
        assert!(config_context.can_work_outside_git());
    }

    #[test]
    fn test_help_detection() {
        let help_context = CompletionContext::new(
            vec![
                "para".to_string(),
                "start".to_string(),
                "--help".to_string(),
            ],
            2,
        );
        assert!(help_context.should_show_help());
        assert_eq!(help_context.get_help_context(), Some("start".to_string()));

        let help_context2 = CompletionContext::new(vec!["para".to_string(), "help".to_string()], 1);
        assert!(help_context2.should_show_help());
    }

    #[test]
    fn test_environment_warnings() {
        let mut context = CompletionContext::new(vec!["para".to_string(), "start".to_string()], 1);
        context.is_git_repository = false;

        let warnings = context.get_environment_warnings();
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("Git repository")));
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
            is_para_session: false,
            current_session: None,
            current_branch: None,
        };

        let file_suggestions = completion.get_file_completions(&context);
        assert!(!file_suggestions.is_empty());
        assert!(file_suggestions.iter().any(|s| s.text == "test.txt"));
        assert!(file_suggestions.iter().any(|s| s.text == "subdir/"));
    }

    #[test]
    fn test_completion_timeout() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let completion = dynamic::DynamicCompletion::new(config)
            .with_timeout(std::time::Duration::from_millis(10));

        let context = CompletionContext::new(vec!["para".to_string(), "start".to_string()], 1);

        let suggestions = completion.get_completions_timeout(&context);
        // Should return some suggestions even with timeout
        assert!(!suggestions.is_empty());
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

#[cfg(test)]
mod cached_completion_tests {
    use super::*;

    #[test]
    fn test_cached_completion_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let mut cached_completion = dynamic::CachedDynamicCompletion::new(config)
            .with_cache_duration(std::time::Duration::from_millis(100));

        cached_completion.clear_cache();

        // First call should populate cache
        let _result1 = cached_completion.get_cached_sessions(false);

        // Second call should use cache
        let _result2 = cached_completion.get_cached_sessions(false);

        // After timeout, cache should be refreshed
        std::thread::sleep(std::time::Duration::from_millis(150));
        let _result3 = cached_completion.get_cached_sessions(false);
    }

    #[test]
    fn test_cache_validity() {
        let timestamp = std::time::Instant::now();
        let duration = std::time::Duration::from_millis(100);

        assert!(dynamic::CachedDynamicCompletion::is_cache_valid(
            timestamp, duration
        ));

        std::thread::sleep(std::time::Duration::from_millis(150));
        assert!(!dynamic::CachedDynamicCompletion::is_cache_valid(
            timestamp, duration
        ));
    }
}
