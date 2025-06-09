use super::{CompletionContext, CompletionProvider, CompletionSuggestion, CompletionType};
use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::Result;
use std::time::Duration;

pub struct DynamicCompletion {
    config: Config,
    timeout: Duration,
}

impl DynamicCompletion {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            timeout: Duration::from_millis(1000),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn get_session_completions(&self, include_archived: bool) -> Result<Vec<CompletionSuggestion>> {
        let session_manager = SessionManager::new(&self.config);
        let sessions = session_manager.list_sessions()?;

        let mut suggestions = Vec::new();

        for session in sessions {
            let should_include = match session.status {
                SessionStatus::Active => true,
                SessionStatus::Finished => include_archived,
                SessionStatus::Cancelled => include_archived,
            };

            if should_include {
                let description = match session.status {
                    SessionStatus::Active => format!("Active session on branch {}", session.branch),
                    SessionStatus::Finished => format!("Finished session ({})", session.updated_at),
                    SessionStatus::Cancelled => {
                        format!("Cancelled session ({})", session.updated_at)
                    }
                };

                suggestions.push(
                    CompletionSuggestion::new(session.name, CompletionType::SessionName)
                        .with_description(description),
                );
            }
        }

        suggestions.sort_by(|a, b| a.text.cmp(&b.text));
        Ok(suggestions)
    }

    fn get_branch_completions(&self) -> Result<Vec<CompletionSuggestion>> {
        let git_service = GitService::discover()?;
        let branches = git_service.list_branches()?;

        let mut suggestions = Vec::new();

        for branch in branches {
            let description = if branch.is_current {
                "Current branch".to_string()
            } else if branch.name.starts_with("remotes/") {
                "Remote branch".to_string()
            } else {
                "Local branch".to_string()
            };

            suggestions.push(
                CompletionSuggestion::new(branch.name, CompletionType::BranchName)
                    .with_description(description),
            );
        }

        suggestions.sort_by(|a, b| a.text.cmp(&b.text));
        Ok(suggestions)
    }

    pub fn get_subcommand_completions(&self) -> Vec<CompletionSuggestion> {
        vec![
            CompletionSuggestion::new("start".to_string(), CompletionType::Subcommand)
                .with_description("Create session with optional name".to_string()),
            CompletionSuggestion::new("dispatch".to_string(), CompletionType::Subcommand)
                .with_description("Start Claude Code session with prompt".to_string()),
            CompletionSuggestion::new("finish".to_string(), CompletionType::Subcommand)
                .with_description("Squash all changes into single commit".to_string()),
            CompletionSuggestion::new("integrate".to_string(), CompletionType::Subcommand)
                .with_description("Squash commits and merge into base branch".to_string()),
            CompletionSuggestion::new("cancel".to_string(), CompletionType::Subcommand)
                .with_description("Cancel session (moves to archive)".to_string()),
            CompletionSuggestion::new("clean".to_string(), CompletionType::Subcommand)
                .with_description("Remove all active sessions".to_string()),
            CompletionSuggestion::new("list".to_string(), CompletionType::Subcommand)
                .with_description("List active sessions".to_string()),
            CompletionSuggestion::new("resume".to_string(), CompletionType::Subcommand)
                .with_description("Resume session in IDE".to_string()),
            CompletionSuggestion::new("recover".to_string(), CompletionType::Subcommand)
                .with_description("Recover cancelled session from archive".to_string()),
            CompletionSuggestion::new("continue".to_string(), CompletionType::Subcommand)
                .with_description("Complete merge after resolving conflicts".to_string()),
            CompletionSuggestion::new("config".to_string(), CompletionType::Subcommand)
                .with_description("Setup configuration".to_string()),
            CompletionSuggestion::new("completion".to_string(), CompletionType::Subcommand)
                .with_description("Generate shell completion script".to_string()),
        ]
    }

    pub fn get_flag_completions(&self, subcommand: Option<&str>) -> Vec<CompletionSuggestion> {
        let mut suggestions = Vec::new();

        match subcommand {
            Some("start") => {
                suggestions.push(
                    CompletionSuggestion::new(
                        "--dangerously-skip-permissions".to_string(),
                        CompletionType::Flag,
                    )
                    .with_description("Skip IDE permission warnings (dangerous)".to_string()),
                );
            }
            Some("dispatch") => {
                suggestions.extend(vec![
                    CompletionSuggestion::new("--file".to_string(), CompletionType::Flag)
                        .with_description("Read prompt from specified file".to_string()),
                    CompletionSuggestion::new("-f".to_string(), CompletionType::Flag)
                        .with_description("Read prompt from specified file".to_string()),
                    CompletionSuggestion::new(
                        "--dangerously-skip-permissions".to_string(),
                        CompletionType::Flag,
                    )
                    .with_description("Skip IDE permission warnings (dangerous)".to_string()),
                ]);
            }
            Some("finish") => {
                suggestions.extend(vec![
                    CompletionSuggestion::new("--branch".to_string(), CompletionType::Flag)
                        .with_description("Rename feature branch to specified name".to_string()),
                    CompletionSuggestion::new("--integrate".to_string(), CompletionType::Flag)
                        .with_description("Automatically integrate into base branch".to_string()),
                    CompletionSuggestion::new("-i".to_string(), CompletionType::Flag)
                        .with_description("Automatically integrate into base branch".to_string()),
                ]);
            }
            Some("list") => {
                suggestions.extend(vec![
                    CompletionSuggestion::new("--verbose".to_string(), CompletionType::Flag)
                        .with_description("Show verbose session information".to_string()),
                    CompletionSuggestion::new("-v".to_string(), CompletionType::Flag)
                        .with_description("Show verbose session information".to_string()),
                    CompletionSuggestion::new("--archived".to_string(), CompletionType::Flag)
                        .with_description("Show archived sessions".to_string()),
                    CompletionSuggestion::new("-a".to_string(), CompletionType::Flag)
                        .with_description("Show archived sessions".to_string()),
                ]);
            }
            Some("clean") => {
                suggestions.push(
                    CompletionSuggestion::new("--backups".to_string(), CompletionType::Flag)
                        .with_description("Also remove archived sessions".to_string()),
                );
            }
            Some("completion") => {
                suggestions.extend(vec![
                    CompletionSuggestion::new("bash".to_string(), CompletionType::Value)
                        .with_description("Generate Bash completion script".to_string()),
                    CompletionSuggestion::new("zsh".to_string(), CompletionType::Value)
                        .with_description("Generate Zsh completion script".to_string()),
                    CompletionSuggestion::new("fish".to_string(), CompletionType::Value)
                        .with_description("Generate Fish completion script".to_string()),
                    CompletionSuggestion::new("powershell".to_string(), CompletionType::Value)
                        .with_description("Generate PowerShell completion script".to_string()),
                ]);
            }
            _ => {}
        }

        suggestions
    }

    pub fn get_file_completions(&self, context: &CompletionContext) -> Vec<CompletionSuggestion> {
        context
            .get_file_completions()
            .into_iter()
            .map(|file| {
                let completion_type = if file.ends_with('/') {
                    CompletionType::DirectoryName
                } else {
                    CompletionType::FileName
                };
                CompletionSuggestion::new(file, completion_type)
            })
            .collect()
    }

    pub fn get_completions_for_context(
        &self,
        context: &CompletionContext,
    ) -> Result<Vec<CompletionSuggestion>> {
        let mut suggestions = Vec::new();

        if context.is_completing_flag() {
            suggestions.extend(self.get_flag_completions(context.get_subcommand()));
        } else if context.is_completing_file() {
            suggestions.extend(self.get_file_completions(context));
        } else if context.is_completing_branch() {
            if context.is_git_repository {
                match self.get_branch_completions() {
                    Ok(branches) => suggestions.extend(branches),
                    Err(_) => {} // Ignore git errors for completion
                }
            }
        } else if context.is_completing_session() {
            let include_archived = context.should_complete_archived_sessions();
            match self.get_session_completions(include_archived) {
                Ok(sessions) => suggestions.extend(sessions),
                Err(_) => {} // Ignore session errors for completion
            }
        } else if context.position == 1 {
            suggestions.extend(self.get_subcommand_completions());
        }

        self.filter_and_sort(&mut suggestions, &context.current_word);
        Ok(suggestions)
    }

    pub fn filter_and_sort(&self, suggestions: &mut Vec<CompletionSuggestion>, prefix: &str) {
        if !prefix.is_empty() {
            suggestions.retain(|suggestion| {
                suggestion.text.starts_with(prefix)
                    || suggestion
                        .text
                        .to_lowercase()
                        .starts_with(&prefix.to_lowercase())
            });
        }

        suggestions.sort_by(|a, b| {
            // Prioritize exact prefix matches
            let a_exact = a.text.starts_with(prefix);
            let b_exact = b.text.starts_with(prefix);

            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.text.cmp(&b.text),
            }
        });
    }

    pub fn get_completions_timeout(
        &self,
        context: &CompletionContext,
    ) -> Vec<CompletionSuggestion> {
        let _timeout = self.timeout;
        let context = context.clone();
        let config = self.config.clone();

        std::thread::scope(|s| {
            let handle = s.spawn(move || {
                let completion = DynamicCompletion::new(config);
                completion.get_completions_for_context(&context)
            });

            match handle.join() {
                Ok(Ok(suggestions)) => suggestions,
                _ => Vec::new(), // Return empty on timeout or error
            }
        })
    }
}

impl CompletionProvider for DynamicCompletion {
    fn get_completions(&self, context: &CompletionContext) -> Result<Vec<String>> {
        let suggestions = self.get_completions_for_context(context)?;
        Ok(suggestions.into_iter().map(|s| s.text).collect())
    }
}

pub struct CachedDynamicCompletion {
    inner: DynamicCompletion,
    session_cache: Option<(std::time::Instant, Vec<CompletionSuggestion>)>,
    branch_cache: Option<(std::time::Instant, Vec<CompletionSuggestion>)>,
    cache_duration: Duration,
}

impl CachedDynamicCompletion {
    pub fn new(config: Config) -> Self {
        Self {
            inner: DynamicCompletion::new(config),
            session_cache: None,
            branch_cache: None,
            cache_duration: Duration::from_secs(5),
        }
    }

    pub fn with_cache_duration(mut self, duration: Duration) -> Self {
        self.cache_duration = duration;
        self
    }

    pub fn is_cache_valid(timestamp: std::time::Instant, duration: Duration) -> bool {
        timestamp.elapsed() < duration
    }

    pub fn get_cached_sessions(
        &mut self,
        include_archived: bool,
    ) -> Result<Vec<CompletionSuggestion>> {
        if let Some((timestamp, ref sessions)) = self.session_cache {
            if Self::is_cache_valid(timestamp, self.cache_duration) {
                return Ok(sessions.clone());
            }
        }

        let sessions = self.inner.get_session_completions(include_archived)?;
        self.session_cache = Some((std::time::Instant::now(), sessions.clone()));
        Ok(sessions)
    }

    pub fn get_cached_branches(&mut self) -> Result<Vec<CompletionSuggestion>> {
        if let Some((timestamp, ref branches)) = self.branch_cache {
            if Self::is_cache_valid(timestamp, self.cache_duration) {
                return Ok(branches.clone());
            }
        }

        let branches = self.inner.get_branch_completions()?;
        self.branch_cache = Some((std::time::Instant::now(), branches.clone()));
        Ok(branches)
    }

    pub fn clear_cache(&mut self) {
        self.session_cache = None;
        self.branch_cache = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig};
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
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        }
    }

    #[test]
    fn test_subcommand_completions() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let completion = DynamicCompletion::new(config);

        let suggestions = completion.get_subcommand_completions();
        assert!(!suggestions.is_empty());

        let start_suggestion = suggestions.iter().find(|s| s.text == "start");
        assert!(start_suggestion.is_some());
        assert!(start_suggestion.unwrap().description.is_some());
    }

    #[test]
    fn test_flag_completions() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let completion = DynamicCompletion::new(config);

        let suggestions = completion.get_flag_completions(Some("dispatch"));
        assert!(!suggestions.is_empty());

        let file_flag = suggestions.iter().find(|s| s.text == "--file");
        assert!(file_flag.is_some());
    }

    #[test]
    fn test_completion_filtering() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let completion = DynamicCompletion::new(config);

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
    fn test_cached_completion() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let mut cached_completion =
            CachedDynamicCompletion::new(config).with_cache_duration(Duration::from_millis(100));

        cached_completion.clear_cache();

        let _result1 = cached_completion.get_cached_sessions(false);
        let _result2 = cached_completion.get_cached_sessions(false);

        std::thread::sleep(Duration::from_millis(150));
        let _result3 = cached_completion.get_cached_sessions(false);
    }
}
