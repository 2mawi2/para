# Task 2: Error Handling & Core Utilities

## Overview
Implement a comprehensive error handling system and core utility functions that will be used throughout the para-rs application.

## Scope
Build the `src/utils/` module with error types and utility functions:

```
src/utils/
├── mod.rs           // Main utils module interface
├── error.rs         // Error types and handling
├── fs.rs           // File system utilities
├── names.rs        // Name generation (friendly names, etc.)
└── json.rs         // JSON handling utilities
```

## Deliverables

### 1. Error System (`error.rs`)
```rust
#[derive(thiserror::Error, Debug)]
pub enum ParaError {
    #[error("Git operation failed: {message}")]
    GitOperation { message: String },
    
    #[error("Session '{session_id}' not found")]
    SessionNotFound { session_id: String },
    
    #[error("Session '{session_id}' already exists")]
    SessionExists { session_id: String },
    
    #[error("Configuration error: {message}")]
    Config { message: String },
    
    #[error("IDE error: {message}")]
    Ide { message: String },
    
    #[error("Invalid arguments: {message}")]
    InvalidArgs { message: String },
    
    #[error("Repository state error: {message}")]
    RepoState { message: String },
    
    #[error("File operation failed: {path}")]
    FileOperation { path: String },
    
    #[error("File not found: {path}")]
    FileNotFound { path: String },
    
    #[error("IDE not available: {ide}")]
    IdeNotAvailable { ide: String },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ParaError>;

// Error construction helpers
impl ParaError {
    pub fn git_operation(message: impl Into<String>) -> Self { ... }
    pub fn session_not_found(session_id: impl Into<String>) -> Self { ... }
    pub fn config_error(message: impl Into<String>) -> Self { ... }
    // ... other helpers
}
```

### 2. File System Utilities (`fs.rs`)
```rust
// Path manipulation and validation
pub fn ensure_absolute_path(path: &Path) -> PathBuf;
pub fn validate_directory_name(name: &str) -> Result<()>;
pub fn create_dir_if_not_exists(path: &Path) -> Result<()>;
pub fn safe_remove_dir(path: &Path) -> Result<()>;

// File operations with proper error handling
pub fn read_file_content(path: &Path) -> Result<String>;
pub fn write_file_content(path: &Path, content: &str) -> Result<()>;
pub fn copy_directory_contents(src: &Path, dst: &Path) -> Result<()>;

// File detection utilities
pub fn is_file_path(input: &str) -> bool;
pub fn find_git_repository() -> Result<PathBuf>;
pub fn get_xdg_config_dir() -> Result<PathBuf>;
```

### 3. Name Generation (`names.rs`)
```rust
// Friendly name generation (like Docker Compose)
pub fn generate_friendly_name() -> String;
pub fn generate_session_id() -> String;
pub fn generate_timestamp() -> String;

// Name validation and cleaning
pub fn validate_session_name(name: &str) -> Result<()>;
pub fn sanitize_branch_name(name: &str) -> String;
pub fn validate_branch_name(name: &str) -> Result<()>;

// Name lists for generation
const ADJECTIVES: &[&str] = &[
    "agile", "bold", "calm", "deep", "eager", "fast", "keen", "neat",
    "quick", "smart", "swift", "wise", "zesty", "bright", "clever",
    // ... more adjectives
];

const NOUNS: &[&str] = &[
    "alpha", "beta", "gamma", "delta", "omega",
    "aurora", "cosmos", "nebula", "quasar", "pulsar",
    // ... more nouns
];
```

### 4. JSON Utilities (`json.rs`)
```rust
// JSON escaping and manipulation for IDE tasks
pub fn json_escape_string(input: &str) -> String;
pub fn create_vscode_task(label: &str, command: &str, args: &[&str]) -> serde_json::Value;
pub fn create_cursor_task(label: &str, command: &str, args: &[&str]) -> serde_json::Value;

// Pretty printing and formatting
pub fn pretty_print_json(value: &serde_json::Value) -> Result<String>;
pub fn minify_json(value: &serde_json::Value) -> Result<String>;
```

### 5. Main Module Interface (`mod.rs`)
```rust
// Re-exports for easy access
pub use error::{ParaError, Result};
pub use fs::*;
pub use names::*;
pub use json::*;

// Common result types
pub type CommandResult = Result<()>;
pub type StringResult = Result<String>;
pub type PathResult = Result<PathBuf>;
```

## Dependencies
```toml
# Add to Cargo.toml
thiserror = "1.0"
anyhow = "1.0"
serde_json = "1.0"
directories = "5.0"
regex = "1.0"
```

## Testing Approach
- Comprehensive unit tests for all utility functions
- Error propagation tests
- Name generation randomness and uniqueness tests
- File system operation tests with temporary directories
- JSON escaping tests with edge cases
- Cross-platform path handling tests

## Acceptance Criteria
✅ All error types are properly defined with helpful messages  
✅ Error chaining works correctly with `anyhow` integration  
✅ File system operations handle permissions and edge cases  
✅ Name generation produces valid, unique, human-readable names  
✅ JSON utilities properly escape special characters  
✅ Path utilities work correctly across platforms  
✅ All functions have comprehensive error handling  
✅ Test coverage is >90% for all utility functions  
✅ Error messages are actionable and user-friendly  

## Interface Design
```rust
// Other modules will import and use like this:
use crate::utils::{ParaError, Result, generate_session_id, read_file_content};

// Functions should be designed for easy use:
let session_id = generate_session_id();
let content = read_file_content(&path)?;
return Err(ParaError::session_not_found("my-session"));
```

## Integration Points
- **All modules**: Every module will use the error types and utilities
- **Config module**: Will use file system utilities and validation
- **Git module**: Will use error types and path utilities
- **CLI module**: Will use error types for user-facing error messages
- **Session module**: Will use name generation and file utilities

## Notes
- This is the most foundational module - other modules depend on it
- Focus on robust error messages that help users understand what went wrong
- Ensure cross-platform compatibility for file system operations
- Keep utility functions simple and well-tested
- Design error types to be easily extended as new features are added