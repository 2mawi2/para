use crate::cli::completion::{CompletionContext, DynamicCompletion};
use crate::cli::parser::CompleteCommandArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;

pub fn execute(args: CompleteCommandArgs) -> Result<()> {
    let command_line: Vec<String> = args.command_line
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let context = CompletionContext {
        command_line,
        current_word: args.current_word,
        previous_word: args.previous_word,
        position: args.position,
        working_directory: std::env::current_dir().unwrap_or_default(),
        is_git_repository: GitService::discover().is_ok(),
        is_para_session: false, // Will be detected in context
        current_session: None,  // Will be detected in context
        current_branch: None,   // Will be detected in context
    };

    let config = Config::load_or_create()?;
    let completion = DynamicCompletion::new(config);
    
    match completion.get_completions_for_context(&context) {
        Ok(suggestions) => {
            for suggestion in suggestions {
                if let Some(description) = suggestion.description {
                    println!("{}:{}", suggestion.text, description);
                } else {
                    println!("{}", suggestion.text);
                }
            }
        }
        Err(_) => {
            // Silent failure for completion - just return empty
        }
    }

    Ok(())
}