use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub command_line: Vec<String>,
    pub current_word: String,
    pub previous_word: Option<String>,
    pub position: usize,
    pub working_directory: PathBuf,
    pub is_git_repository: bool,
}

impl CompletionContext {


    pub fn get_subcommand(&self) -> Option<&str> {
        self.command_line.get(1).map(|s| s.as_str())
    }


    pub fn is_completing_flag(&self) -> bool {
        self.current_word.starts_with('-')
    }

    pub fn is_completing_value_for_flag(&self, flag: &str) -> bool {
        self.previous_word.as_ref().is_some_and(|prev| {
            prev == flag
                || prev == &format!("--{}", flag.trim_start_matches('-'))
                || (flag == "-f" && prev == "--file")
                || (flag == "--file" && prev == "-f")
        })
    }

    pub fn is_completing_file(&self) -> bool {
        self.is_completing_value_for_flag("--file") || self.is_completing_value_for_flag("-f")
    }

    pub fn is_completing_branch(&self) -> bool {
        self.is_completing_value_for_flag("--branch")
    }

    pub fn is_completing_session(&self) -> bool {
        match self.get_subcommand() {
            Some("resume") | Some("cancel") | Some("recover") => {
                self.position >= 2 && !self.is_completing_flag()
            }
            Some("finish") | Some("integrate") => self.position == 3 && !self.is_completing_flag(),
            _ => false,
        }
    }

    pub fn should_complete_archived_sessions(&self) -> bool {
        matches!(self.get_subcommand(), Some("recover"))
    }


    pub fn get_file_completions(&self) -> Vec<String> {
        let mut completions = Vec::new();

        let search_dir = if self.current_word.is_empty() {
            &self.working_directory
        } else {
            Path::new(&self.current_word)
                .parent()
                .unwrap_or(&self.working_directory)
        };

        if let Ok(entries) = std::fs::read_dir(search_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if !name.starts_with('.') {
                        let path_str = if search_dir == self.working_directory {
                            name.to_string()
                        } else {
                            format!("{}/{}", search_dir.display(), name)
                        };

                        if entry.path().is_dir() {
                            completions.push(format!("{}/", path_str));
                        } else {
                            completions.push(path_str);
                        }
                    }
                }
            }
        }

        completions.sort();
        completions
    }

}


#[cfg(test)]
mod tests {
    use super::*;

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
            is_git_repository: false, // For tests, we don't need real git detection
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
    fn test_subcommand_detection() {
        let command_line = vec![
            "para".to_string(),
            "finish".to_string(),
            "message".to_string(),
        ];
        let context = create_test_context(command_line, 2);

        assert_eq!(context.get_subcommand(), Some("finish"));
    }

    #[test]
    fn test_flag_completion_detection() {
        let command_line = vec![
            "para".to_string(),
            "start".to_string(),
            "--branch".to_string(),
        ];
        let context = create_test_context(command_line.clone(), 2);

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
    }

}
