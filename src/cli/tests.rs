#[cfg(test)]
mod cli_tests {
    use crate::cli::parser::*;
    use clap::Parser;

    #[test]
    fn test_start_command_parsing() {
        let cli = Cli::try_parse_from(["para", "start"]).unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert!(args.name_or_session.is_none());
                assert!(!args.dangerously_skip_permissions);
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_start_command_with_name() {
        let cli = Cli::try_parse_from(["para", "start", "my-feature"]).unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert_eq!(args.name_or_session, Some("my-feature".to_string()));
                assert!(!args.dangerously_skip_permissions);
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_start_command_with_dangerous_flag() {
        let cli = Cli::try_parse_from(["para", "start", "--dangerously-skip-permissions"]).unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert!(args.name_or_session.is_none());
                assert!(args.dangerously_skip_permissions);
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_unified_start_with_prompt() {
        let cli = Cli::try_parse_from(["para", "start", "implement feature"]).unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert_eq!(args.name_or_session, Some("implement feature".to_string()));
                assert!(args.prompt.is_none());
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_unified_start_with_session_and_prompt() {
        let cli =
            Cli::try_parse_from(["para", "start", "my-session", "implement feature"]).unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert_eq!(args.name_or_session, Some("my-session".to_string()));
                assert_eq!(args.prompt, Some("implement feature".to_string()));
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_unified_start_with_file() {
        let cli = Cli::try_parse_from(["para", "start", "--file", "task.txt"]).unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert!(args.name_or_session.is_none());
                assert!(args.prompt.is_none());
                assert_eq!(args.file, Some(std::path::PathBuf::from("task.txt")));
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_finish_command_basic() {
        let cli = Cli::try_parse_from(["para", "finish", "Complete feature"]).unwrap();
        match cli.command.unwrap() {
            Commands::Finish(args) => {
                assert_eq!(args.message, "Complete feature");
                assert!(args.branch.is_none());
                assert!(args.session.is_none());
            }
            _ => panic!("Expected Finish command"),
        }
    }

    #[test]
    fn test_finish_command_with_branch() {
        let cli = Cli::try_parse_from([
            "para",
            "finish",
            "Complete feature",
            "--branch",
            "my-branch",
        ])
        .unwrap();
        match cli.command.unwrap() {
            Commands::Finish(args) => {
                assert_eq!(args.message, "Complete feature");
                assert_eq!(args.branch, Some("my-branch".to_string()));
            }
            _ => panic!("Expected Finish command"),
        }
    }

    #[test]
    fn test_list_command_alias() {
        let cli = Cli::try_parse_from(["para", "ls"]).unwrap();
        match cli.command.unwrap() {
            Commands::List(_) => {}
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_list_command_with_verbose() {
        let cli = Cli::try_parse_from(["para", "list", "--verbose"]).unwrap();
        match cli.command.unwrap() {
            Commands::List(args) => {
                assert!(args.verbose);
                assert!(!args.archived);
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_config_subcommands() {
        let cli = Cli::try_parse_from(["para", "config", "setup"]).unwrap();
        match cli.command.unwrap() {
            Commands::Config(args) => match args.command.unwrap() {
                ConfigCommands::Setup => {}
                _ => panic!("Expected Setup subcommand"),
            },
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_completion_command() {
        let cli = Cli::try_parse_from(["para", "completion", "bash"]).unwrap();
        match cli.command.unwrap() {
            Commands::Completion(args) => {
                assert_eq!(args.shell, "bash");
            }
            _ => panic!("Expected Completion command"),
        }
    }

    #[test]
    fn test_session_name_validation() {
        use crate::cli::parser::validate_session_name;

        assert!(validate_session_name("valid-name").is_ok());
        assert!(validate_session_name("valid_name").is_ok());
        assert!(validate_session_name("valid123").is_ok());

        assert!(validate_session_name("").is_err());
        assert!(validate_session_name("invalid name").is_err());
        assert!(validate_session_name("invalid@name").is_err());

        let long_name = "a".repeat(51);
        assert!(validate_session_name(&long_name).is_err());
    }

    #[test]
    fn test_branch_name_validation() {
        use crate::cli::parser::validate_branch_name;

        assert!(validate_branch_name("valid-branch").is_ok());
        assert!(validate_branch_name("feature/auth").is_ok());

        assert!(validate_branch_name("").is_err());
        assert!(validate_branch_name("-invalid").is_err());
        assert!(validate_branch_name("invalid-").is_err());
        assert!(validate_branch_name("invalid..name").is_err());
        assert!(validate_branch_name("invalid//name").is_err());
    }

    #[test]
    fn test_finish_args_validation() {
        let args = FinishArgs {
            message: "".to_string(),
            branch: None,
            session: None,
        };
        assert!(args.validate().is_err());

        let args = FinishArgs {
            message: "Valid commit message".to_string(),
            branch: None,
            session: None,
        };
        assert!(args.validate().is_ok());

        let args = FinishArgs {
            message: "Valid commit message".to_string(),
            branch: Some("-invalid".to_string()),
            session: None,
        };
        assert!(args.validate().is_err());
    }

    #[test]
    fn test_completion_sessions_command() {
        let cli = Cli::try_parse_from(["para", "_completion_sessions"]).unwrap();
        match cli.command.unwrap() {
            Commands::CompletionSessions => {}
            _ => panic!("Expected CompletionSessions command"),
        }
    }

    #[test]
    fn test_completion_branches_command() {
        let cli = Cli::try_parse_from(["para", "_completion_branches"]).unwrap();
        match cli.command.unwrap() {
            Commands::CompletionBranches => {}
            _ => panic!("Expected CompletionBranches command"),
        }
    }

    #[test]
    fn test_completion_commands_are_hidden() {
        use clap::CommandFactory;
        let app = Cli::command();

        let completion_sessions_cmd = app.find_subcommand("_completion_sessions").unwrap();
        assert!(completion_sessions_cmd.is_hide_set());

        let completion_branches_cmd = app.find_subcommand("_completion_branches").unwrap();
        assert!(completion_branches_cmd.is_hide_set());
    }

    #[test]
    fn test_unified_start_command_with_prompt() {
        let cli = Cli::try_parse_from(["para", "start", "Add user authentication"]).unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert_eq!(
                    args.name_or_session,
                    Some("Add user authentication".to_string())
                );
                assert!(args.prompt.is_none());
                assert!(args.file.is_none());
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_unified_start_command_with_name_and_prompt() {
        let cli =
            Cli::try_parse_from(["para", "start", "feature-name", "Add authentication"]).unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert_eq!(args.name_or_session, Some("feature-name".to_string()));
                assert_eq!(args.prompt, Some("Add authentication".to_string()));
                assert!(args.file.is_none());
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_unified_start_command_with_file() {
        let cli = Cli::try_parse_from(["para", "start", "--file", "prompt.txt"]).unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert!(args.name_or_session.is_none());
                assert!(args.prompt.is_none());
                assert_eq!(args.file, Some(std::path::PathBuf::from("prompt.txt")));
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_unified_start_args_validation() {
        use crate::cli::parser::{SandboxArgs, UnifiedStartArgs};

        // Test that prompt and file conflict
        let args = UnifiedStartArgs {
            name_or_session: None,
            prompt: Some("test prompt".to_string()),
            file: Some(std::path::PathBuf::from("test.txt")),
            dangerously_skip_permissions: false,
            container: false,
            allow_domains: None,
            docker_args: vec![],
            setup_script: None,
            docker_image: None,
            no_forward_keys: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
            },
        };
        assert!(args.validate().is_err());

        // Test that sandbox flags conflict
        let args = UnifiedStartArgs {
            name_or_session: None,
            prompt: Some("test prompt".to_string()),
            file: None,
            dangerously_skip_permissions: false,
            container: false,
            allow_domains: None,
            docker_args: vec![],
            setup_script: None,
            docker_image: None,
            no_forward_keys: false,
            sandbox_args: SandboxArgs {
                sandbox: true,
                no_sandbox: true,
                sandbox_profile: None,
            },
        };
        assert!(args.validate().is_err());

        // Test valid args
        let args = UnifiedStartArgs {
            name_or_session: Some("test-session".to_string()),
            prompt: Some("test prompt".to_string()),
            file: None,
            dangerously_skip_permissions: false,
            container: false,
            allow_domains: None,
            docker_args: vec![],
            setup_script: None,
            docker_image: None,
            no_forward_keys: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
            },
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_completion_init_user_expectation() {
        // Test that users can naturally do "para completion init"
        // since we tell them to run "para init" from the completion command
        let result = Cli::try_parse_from(["para", "completion", "init"]);

        // This should now work - "init" is accepted as a shell string
        assert!(
            result.is_ok(),
            "para completion init should be a valid command"
        );

        let cli = result.unwrap();
        match cli.command.unwrap() {
            Commands::Completion(args) => {
                // Verify that "init" was parsed as the shell string
                assert_eq!(args.shell, "init");
            }
            _ => panic!("Expected Completion command"),
        }
    }

    #[test]
    fn test_unified_start_resume_with_prompt() {
        // Test resuming a session with additional prompt
        let cli = Cli::try_parse_from(["para", "start", "existing-session", "add error handling"])
            .unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert_eq!(args.name_or_session, Some("existing-session".to_string()));
                assert_eq!(args.prompt, Some("add error handling".to_string()));
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_unified_start_with_container_and_prompt() {
        // Test container session with prompt
        let cli =
            Cli::try_parse_from(["para", "start", "--container", "api", "implement endpoint"])
                .unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert!(args.container);
                assert_eq!(args.name_or_session, Some("api".to_string()));
                assert_eq!(args.prompt, Some("implement endpoint".to_string()));
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_unified_start_with_sandbox_flags() {
        // Test sandbox configuration
        let cli = Cli::try_parse_from([
            "para",
            "start",
            "--sandbox",
            "--sandbox-profile",
            "restrictive",
            "feature",
        ])
        .unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert!(args.sandbox_args.sandbox);
                assert_eq!(
                    args.sandbox_args.sandbox_profile,
                    Some("restrictive".to_string())
                );
                assert_eq!(args.name_or_session, Some("feature".to_string()));
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_unified_start_current_directory_resume() {
        // Test resuming from current directory (no args)
        let cli = Cli::try_parse_from(["para", "start"]).unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert_eq!(args.name_or_session, None);
                assert_eq!(args.prompt, None);
                assert_eq!(args.file, None);
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_unified_start_docker_advanced_options() {
        // Test Docker-specific options
        let cli = Cli::try_parse_from([
            "para",
            "start",
            "--container",
            "--docker-image",
            "ubuntu:22.04",
            "--allow-domains",
            "api.example.com,cdn.example.com",
            "--no-forward-keys",
            "docker-session",
        ])
        .unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert!(args.container);
                assert_eq!(args.docker_image, Some("ubuntu:22.04".to_string()));
                assert_eq!(
                    args.allow_domains,
                    Some("api.example.com,cdn.example.com".to_string())
                );
                assert!(args.no_forward_keys);
                assert_eq!(args.name_or_session, Some("docker-session".to_string()));
            }
            _ => panic!("Expected Start command"),
        }
    }
}
