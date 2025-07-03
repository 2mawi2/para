#[cfg(test)]
mod generator_tests {
    use crate::cli::completion::generators::ShellCompletionGenerator;
    use crate::cli::parser::Shell;

    #[test]
    fn test_basic_completion_generation() {
        let bash_completion = ShellCompletionGenerator::generate_basic_completion(Shell::Bash);
        assert!(bash_completion.is_ok());
        let bash_script = bash_completion.unwrap();
        assert!(bash_script.contains("para"));
        assert!(bash_script.contains("complete"));

        let zsh_completion = ShellCompletionGenerator::generate_basic_completion(Shell::Zsh);
        assert!(zsh_completion.is_ok());
        let zsh_script = zsh_completion.unwrap();
        assert!(zsh_script.contains("para"));
        assert!(zsh_script.contains("compdef"));

        let fish_completion = ShellCompletionGenerator::generate_basic_completion(Shell::Fish);
        assert!(fish_completion.is_ok());
        let fish_script = fish_completion.unwrap();
        assert!(fish_script.contains("para"));
        assert!(fish_script.contains("complete"));
    }

    #[test]
    fn test_enhanced_completion_generation() {
        let enhanced_bash = ShellCompletionGenerator::generate_enhanced_completion(Shell::Bash);
        assert!(enhanced_bash.is_ok());
        let bash_script = enhanced_bash.unwrap();
        assert!(bash_script.contains("para"));
        assert!(bash_script.contains("_para_complete_sessions"));

        let enhanced_zsh = ShellCompletionGenerator::generate_enhanced_completion(Shell::Zsh);
        assert!(enhanced_zsh.is_ok());
        let zsh_script = enhanced_zsh.unwrap();
        assert!(zsh_script.contains("para"));
        assert!(zsh_script.contains("_para_sessions"));

        let enhanced_fish = ShellCompletionGenerator::generate_enhanced_completion(Shell::Fish);
        assert!(enhanced_fish.is_ok());
        let fish_script = enhanced_fish.unwrap();
        assert!(fish_script.contains("para"));
        assert!(fish_script.contains("__para_sessions"));
        assert!(fish_script.contains("__para_task_files"));
    }

    #[test]
    fn test_installation_instructions() {
        let bash_instructions =
            ShellCompletionGenerator::get_installation_instructions(Shell::Bash);
        assert!(bash_instructions.contains("Installation instructions"));
        assert!(bash_instructions.contains("bash"));
        assert!(bash_instructions.contains("~/.bashrc"));

        let zsh_instructions = ShellCompletionGenerator::get_installation_instructions(Shell::Zsh);
        assert!(zsh_instructions.contains("Installation instructions"));
        assert!(zsh_instructions.contains("zsh"));
        assert!(zsh_instructions.contains("~/.zshrc"));

        let fish_instructions =
            ShellCompletionGenerator::get_installation_instructions(Shell::Fish);
        assert!(fish_instructions.contains("Installation instructions"));
        assert!(fish_instructions.contains("fish"));
        assert!(fish_instructions.contains("completions"));
    }

    #[test]
    fn test_completion_excludes_removed_commands() {
        // Test that the completion script no longer includes integrate/continue commands
        let fish_completion = ShellCompletionGenerator::generate_enhanced_completion(Shell::Fish);
        assert!(fish_completion.is_ok());
        let fish_script = fish_completion.unwrap();

        // Should not contain references to removed commands
        assert!(!fish_script.contains("integrate"));
        assert!(!fish_script.contains("continue"));

        // Should contain existing commands
        assert!(fish_script.contains("start"));
        assert!(fish_script.contains("finish"));
        assert!(fish_script.contains("dispatch"));
        assert!(fish_script.contains("init"));
    }

    #[test]
    fn test_completion_includes_new_init_command() {
        let bash_completion = ShellCompletionGenerator::generate_basic_completion(Shell::Bash);
        assert!(bash_completion.is_ok());
        let bash_script = bash_completion.unwrap();
        assert!(bash_script.contains("init"));

        let fish_completion = ShellCompletionGenerator::generate_basic_completion(Shell::Fish);
        assert!(fish_completion.is_ok());
        let fish_script = fish_completion.unwrap();
        assert!(fish_script.contains("init"));
    }
}

#[cfg(test)]
mod completion_command_tests {
    use crate::cli::commands::completion;
    use crate::cli::parser::CompletionArgs;

    #[test]
    fn test_completion_init_suggestion() {
        let args = CompletionArgs {
            shell: "init".to_string(),
        };

        // This should not panic and should provide helpful guidance
        let result = completion::execute(args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_completion_unsupported_shell() {
        let args = CompletionArgs {
            shell: "unsupported".to_string(),
        };

        // This should handle unsupported shells gracefully
        let result = completion::execute(args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_supported_shells() {
        for shell_name in ["bash", "zsh", "fish"] {
            let args = CompletionArgs {
                shell: shell_name.to_string(),
            };

            let result = completion::execute(args);
            assert!(result.is_ok(), "Shell {shell_name} should be supported");
        }
    }
}
