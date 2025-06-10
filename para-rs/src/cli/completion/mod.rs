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
pub struct CompletionSuggestion {
    pub text: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompletionType {
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
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

