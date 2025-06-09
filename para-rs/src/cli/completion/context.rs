use crate::core::git::{GitOperations, GitService};
use crate::utils::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub command_line: Vec<String>,
    pub current_word: String,
    pub previous_word: Option<String>,
    pub position: usize,
    pub working_directory: PathBuf,
    pub is_git_repository: bool,
    pub is_para_session: bool,
    pub current_session: Option<String>,
    pub current_branch: Option<String>,
}

impl CompletionContext {
    pub fn new(command_line: Vec<String>, position: usize) -> Self {
        let current_word = command_line.get(position).cloned().unwrap_or_default();
        let previous_word = if position > 0 {
            command_line.get(position - 1).cloned()
        } else {
            None
        };

        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let is_git_repository = GitService::discover().is_ok();
        let (is_para_session, current_session, current_branch) = Self::detect_session_context();

        Self {
            command_line,
            current_word,
            previous_word,
            position,
            working_directory,
            is_git_repository,
            is_para_session,
            current_session,
            current_branch,
        }
    }

    pub fn from_env() -> Result<Self> {
        let args: Vec<String> = std::env::args().collect();
        let position = args.len().saturating_sub(1);
        Ok(Self::new(args, position))
    }

    pub fn get_subcommand(&self) -> Option<&str> {
        self.command_line.get(1).map(|s| s.as_str())
    }

    pub fn get_subcommand_args(&self) -> &[String] {
        if self.command_line.len() > 2 {
            &self.command_line[2..]
        } else {
            &[]
        }
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

    pub fn filter_prefix(&self, suggestions: &mut Vec<String>) {
        if self.current_word.is_empty() {
            return;
        }

        suggestions.retain(|suggestion| {
            suggestion.starts_with(&self.current_word)
                || suggestion
                    .to_lowercase()
                    .starts_with(&self.current_word.to_lowercase())
        });
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

    fn detect_session_context() -> (bool, Option<String>, Option<String>) {
        let current_branch = GitService::discover()
            .ok()
            .and_then(|service| service.get_current_branch().ok());

        let is_para_session = current_branch
            .as_ref()
            .map(|branch: &String| branch.starts_with("pc/"))
            .unwrap_or(false);

        let current_session = if is_para_session {
            current_branch
                .as_ref()
                .and_then(|branch| branch.strip_prefix("pc/").map(|s| s.to_string()))
        } else {
            None
        };

        (is_para_session, current_session, current_branch)
    }

    pub fn get_session_from_worktree(&self) -> Option<String> {
        if let Ok(service) = GitService::discover() {
            let worktrees = service.list_worktrees().unwrap_or_default();
            for worktree in worktrees {
                if self.working_directory.starts_with(&worktree.path) {
                    if let Some(session) = worktree.branch.strip_prefix("pc/") {
                        return Some(session.to_string());
                    }
                }
            }
        }
        None
    }

    pub fn should_show_help(&self) -> bool {
        self.current_word == "help" || self.current_word == "--help" || self.current_word == "-h"
    }

    pub fn get_help_context(&self) -> Option<String> {
        if self.should_show_help() {
            self.get_subcommand().map(|s| s.to_string())
        } else {
            None
        }
    }

    pub fn needs_git_repository(&self) -> bool {
        matches!(
            self.get_subcommand(),
            Some("start")
                | Some("dispatch")
                | Some("finish")
                | Some("integrate")
                | Some("cancel")
                | Some("clean")
                | Some("list")
                | Some("resume")
                | Some("recover")
                | Some("continue")
        )
    }

    pub fn can_work_outside_git(&self) -> bool {
        matches!(
            self.get_subcommand(),
            Some("config") | Some("completion") | Some("help") | None
        )
    }

    pub fn get_environment_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.needs_git_repository() && !self.is_git_repository {
            warnings.push("This command requires a Git repository".to_string());
        }

        if self.get_subcommand().is_some()
            && !self.can_work_outside_git()
            && !self.is_git_repository
        {
            warnings.push("Para commands work best inside a Git repository".to_string());
        }

        warnings
    }

    pub fn get_repository_root(&self) -> Option<PathBuf> {
        GitService::discover()
            .ok()
            .map(|service| service.repository().root.clone())
    }

    pub fn is_in_worktree(&self) -> bool {
        if let Ok(service) = GitService::discover() {
            let worktrees = service.list_worktrees().unwrap_or_default();
            worktrees
                .iter()
                .any(|wt| self.working_directory.starts_with(&wt.path))
        } else {
            false
        }
    }

    pub fn get_completion_type(&self) -> CompletionType {
        if self.is_completing_flag() {
            CompletionType::Flag
        } else if self.is_completing_file() {
            CompletionType::File
        } else if self.is_completing_branch() {
            CompletionType::Branch
        } else if self.is_completing_session() {
            CompletionType::Session
        } else if self.position == 1 {
            CompletionType::Subcommand
        } else {
            CompletionType::Argument
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompletionType {
    Subcommand,
    Flag,
    File,
    Branch,
    Session,
    Argument,
}

#[cfg(test)]
mod tests {
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
    fn test_flag_completion_detection() {
        let command_line = vec![
            "para".to_string(),
            "start".to_string(),
            "--branch".to_string(),
        ];
        let context = CompletionContext::new(command_line.clone(), 2);

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
        assert_eq!(flag_context.get_completion_type(), CompletionType::Flag);

        let subcommand_context =
            CompletionContext::new(vec!["para".to_string(), "sta".to_string()], 1);
        assert_eq!(
            subcommand_context.get_completion_type(),
            CompletionType::Subcommand
        );

        let session_context = CompletionContext::new(
            vec!["para".to_string(), "resume".to_string(), "sess".to_string()],
            2,
        );
        assert_eq!(
            session_context.get_completion_type(),
            CompletionType::Session
        );
    }
}
