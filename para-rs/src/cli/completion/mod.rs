pub mod context;
pub mod dynamic;
pub mod generators;

#[cfg(test)]
mod tests;

pub use context::CompletionContext;
pub use dynamic::DynamicCompletion;

use crate::utils::Result;

pub trait CompletionProvider {
    fn get_completions(&self, context: &CompletionContext) -> Result<Vec<String>>;
}

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub shell: String,
    pub command_line: Vec<String>,
    pub current_word: String,
    pub previous_word: Option<String>,
    pub position: usize,
}

#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub suggestions: Vec<CompletionSuggestion>,
    pub completion_type: CompletionType,
}

#[derive(Debug, Clone)]
pub struct CompletionSuggestion {
    pub text: String,
    pub description: Option<String>,
    pub completion_type: CompletionType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompletionType {
    Command,
    Subcommand,
    Flag,
    SessionName,
    BranchName,
    FileName,
    DirectoryName,
    Value,
}

impl CompletionSuggestion {
    pub fn new(text: String, completion_type: CompletionType) -> Self {
        Self {
            text,
            description: None,
            completion_type,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

impl CompletionResponse {
    pub fn new(suggestions: Vec<CompletionSuggestion>, completion_type: CompletionType) -> Self {
        Self {
            suggestions,
            completion_type,
        }
    }

    pub fn empty() -> Self {
        Self {
            suggestions: Vec::new(),
            completion_type: CompletionType::Value,
        }
    }

    pub fn filter_by_prefix(&mut self, prefix: &str) {
        if prefix.is_empty() {
            return;
        }

        self.suggestions.retain(|suggestion| {
            suggestion.text.starts_with(prefix)
                || suggestion
                    .text
                    .to_lowercase()
                    .starts_with(&prefix.to_lowercase())
        });
    }

    pub fn sort(&mut self) {
        self.suggestions.sort_by(|a, b| a.text.cmp(&b.text));
    }

    pub fn limit(&mut self, max_suggestions: usize) {
        if self.suggestions.len() > max_suggestions {
            self.suggestions.truncate(max_suggestions);
        }
    }
}
