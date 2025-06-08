# Task 1: Configuration Management System

## Overview
Implement a complete configuration management system for para-rs that handles loading, validation, saving, and interactive setup of user preferences.

## Scope
Build the `src/config/` module with all configuration-related functionality:

```
src/config/
├── mod.rs           // Main config module interface
├── manager.rs       // Configuration loading/saving
├── wizard.rs        // Interactive configuration setup  
├── validation.rs    // Configuration validation
└── defaults.rs      // Default values and IDE detection
```

## Deliverables

### 1. Core Configuration Types (`mod.rs`)
```rust
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Config {
    pub ide: IdeConfig,
    pub directories: DirectoryConfig,
    pub git: GitConfig,
    pub session: SessionConfig,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct IdeConfig {
    pub name: String,
    pub command: String,
    pub user_data_dir: Option<String>,
    pub wrapper: WrapperConfig,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct WrapperConfig {
    pub enabled: bool,
    pub name: String,
    pub command: String,
}

// Additional config structs for directories, git, session settings
```

### 2. Configuration Manager (`manager.rs`)
- `Config::load_or_create() -> Result<Config>`
- `Config::save(&self) -> Result<()>`
- `Config::validate(&self) -> Result<()>`
- XDG-compliant config file location using `directories-rs`
- Environment variable override support
- Backward compatibility with shell version config format

### 3. Interactive Wizard (`wizard.rs`)
- `run_config_wizard() -> Result<Config>`
- Auto-detection of installed IDEs (cursor, code, claude)
- Interactive prompts using `dialoguer` crate
- Validation of user inputs
- Preview and confirmation before saving

### 4. Validation System (`validation.rs`)
- IDE command validation (check if executable exists)
- Directory name validation (no path separators, valid characters)
- Branch prefix validation (Git-compatible names)
- Comprehensive error messages with suggestions

### 5. Defaults & Auto-Detection (`defaults.rs`)
- Default configuration values
- IDE auto-detection logic (check common installation paths)
- OS-specific default paths
- Fallback configurations

## Dependencies
```toml
# Add to Cargo.toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
directories = "5.0"
dialoguer = "0.11"
```

## Interface Design
```rust
// Public API that other modules will use
pub use config::{Config, IdeConfig, DirectoryConfig};

// Main functions other modules need
impl Config {
    pub fn load_or_create() -> Result<Self>;
    pub fn get_ide_command(&self) -> &str;
    pub fn get_branch_prefix(&self) -> &str;
    pub fn is_wrapper_enabled(&self) -> bool;
}
```

## Testing Approach
- Unit tests for each validation function
- Integration tests for config loading/saving
- Mock file system for testing XDG directories
- Test auto-detection logic with mock environments
- Test wizard with simulated user inputs

## Acceptance Criteria
✅ Can load configuration from XDG-compliant location  
✅ Falls back to sensible defaults when no config exists  
✅ Interactive wizard can detect and configure IDEs  
✅ Validates all configuration values with helpful errors  
✅ Supports environment variable overrides  
✅ Backward compatible with shell version config format  
✅ All functions are thoroughly tested  
✅ Configuration can be serialized/deserialized correctly  

## Integration Points
- **Error types**: Will use placeholder error types initially, integrate with error system later
- **CLI**: Will be called by `para config` command once CLI is implemented
- **IDE module**: Will provide IDE configuration data
- **Git module**: Will provide git-related configuration

## Notes
- This module is foundational but self-contained
- Can be developed and tested independently
- Should focus on robust validation and user experience
- Keep the public API simple and clean for other modules to use