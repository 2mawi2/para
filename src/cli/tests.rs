#[cfg(test)]
mod cli_tests {
    use crate::cli::parser::*;
    use clap::Parser;

    #[test]
    fn test_start_command_parsing() {
        let cli = Cli::try_parse_from(["para", "start"]).unwrap();
        match cli.command.unwrap() {
            Commands::Start(args) => {
                assert!(args.name.is_none());
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
                assert_eq!(args.name, Some("my-feature".to_string()));
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
                assert!(args.name.is_none());
                assert!(args.dangerously_skip_permissions);
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_dispatch_command_with_prompt() {
        let cli = Cli::try_parse_from(["para", "dispatch", "Add user authentication"]).unwrap();
        match cli.command.unwrap() {
            Commands::Dispatch(args) => {
                assert_eq!(
                    args.name_or_prompt,
                    Some("Add user authentication".to_string())
                );
                assert!(args.prompt.is_none());
                assert!(args.file.is_none());
            }
            _ => panic!("Expected Dispatch command"),
        }
    }

    #[test]
    fn test_dispatch_command_with_file() {
        let cli = Cli::try_parse_from(["para", "dispatch", "--file", "prompt.txt"]).unwrap();
        match cli.command.unwrap() {
            Commands::Dispatch(args) => {
                assert!(args.name_or_prompt.is_none());
                assert!(args.prompt.is_none());
                assert_eq!(args.file, Some(std::path::PathBuf::from("prompt.txt")));
            }
            _ => panic!("Expected Dispatch command"),
        }
    }

    #[test]
    fn test_finish_command_basic() {
        let cli = Cli::try_parse_from(["para", "finish", "Complete feature"]).unwrap();
        match cli.command.unwrap() {
            Commands::Finish(args) => {
                assert_eq!(args.message, "Complete feature");
                assert!(args.branch.is_none());
                assert!(!args.integrate);
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
                assert!(!args.integrate);
            }
            _ => panic!("Expected Finish command"),
        }
    }

    #[test]
    fn test_finish_command_with_integrate() {
        let cli =
            Cli::try_parse_from(["para", "finish", "Complete feature", "--integrate"]).unwrap();
        match cli.command.unwrap() {
            Commands::Finish(args) => {
                assert_eq!(args.message, "Complete feature");
                assert!(args.integrate);
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
            Commands::Completion(args) => match args.shell {
                Shell::Bash => {}
                _ => panic!("Expected Bash shell"),
            },
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
    fn test_dispatch_args_validation() {
        let args = DispatchArgs {
            name_or_prompt: None,
            prompt: None,
            file: None,
            description: None,
            dangerously_skip_permissions: false,
        };
        assert!(args.validate().is_err());

        let args = DispatchArgs {
            name_or_prompt: Some("test prompt".to_string()),
            prompt: None,
            file: None,
            description: None,
            dangerously_skip_permissions: false,
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_finish_args_validation() {
        let args = FinishArgs {
            message: "".to_string(),
            branch: None,
            integrate: false,
            session: None,
        };
        assert!(args.validate().is_err());

        let args = FinishArgs {
            message: "Valid commit message".to_string(),
            branch: None,
            integrate: false,
            session: None,
        };
        assert!(args.validate().is_ok());

        let args = FinishArgs {
            message: "Valid commit message".to_string(),
            branch: Some("-invalid".to_string()),
            integrate: false,
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
    fn test_complete_command_args() {
        let cli = Cli::try_parse_from([
            "para",
            "complete-command",
            "--command-line",
            "para start",
            "--current-word",
            "my-",
            "--position",
            "2",
        ])
        .unwrap();
        match cli.command.unwrap() {
            Commands::CompleteCommand(args) => {
                assert_eq!(args.command_line, "para start");
                assert_eq!(args.current_word, "my-");
                assert_eq!(args.position, 2);
                assert!(args.previous_word.is_none());
            }
            _ => panic!("Expected CompleteCommand command"),
        }
    }

    #[test]
    fn test_complete_command_with_previous_word() {
        let cli = Cli::try_parse_from([
            "para",
            "complete-command",
            "--command-line",
            "para finish my message",
            "--current-word",
            "message",
            "--previous-word",
            "my",
            "--position",
            "3",
        ])
        .unwrap();
        match cli.command.unwrap() {
            Commands::CompleteCommand(args) => {
                assert_eq!(args.command_line, "para finish my message");
                assert_eq!(args.current_word, "message");
                assert_eq!(args.previous_word, Some("my".to_string()));
                assert_eq!(args.position, 3);
            }
            _ => panic!("Expected CompleteCommand command"),
        }
    }
}
