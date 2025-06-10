# CLAUDE.md

This file provides guidance for working with the Rust implementation of Para.

## Project Overview

Para-rs is the Rust rewrite of Para - a "Parallel IDE Workflow Helper" that enables developers to work on multiple features simultaneously using Git worktrees and IDEs. This Rust implementation aims to provide better performance, cross-platform compatibility, and enhanced reliability.

## Essential Development Commands

### Setup and Testing
```bash
just test             # Run all Rust checks (tests + linting + formatting)
just test-unit        # Run Rust unit tests only
just test-legacy      # Run legacy bats tests against Rust binary
just lint             # Run clippy linting
just fmt              # Auto-fix Rust formatting
just fmt-check        # Check Rust formatting
```

### Build Commands
```bash
cargo build           # Build debug binary
cargo build --release # Build optimized release binary
cargo run -- --help   # Run with arguments
```

## Architecture Overview

### Modular Rust Design
```
src/
├── cli/              # Command-line interface, argument parsing, and command implementations
│   └── commands/     # Individual command implementations (start, finish, dispatch, etc.)
├── config/           # Configuration management, validation, and interactive wizard
├── core/             # Core business logic, session management, and IDE integration
│   └── git/          # Git operations, worktree management, and repository handling
├── utils/            # Utility functions, error handling, and helper modules
└── main.rs           # Application entry point
```

### Key Dependencies
- **clap**: Command-line argument parsing with completion support
- **serde/serde_json**: Configuration serialization
- **dialoguer**: Interactive prompts and wizards
- **chrono**: Date/time handling for sessions
- **anyhow/thiserror**: Error handling
- **directories**: Cross-platform config directories

## Code Style Requirements

- **Error handling**: Use `anyhow::Result` for application errors, `thiserror` for custom error types
- **CLI design**: Follow clap conventions with derive API
- **Configuration**: Use serde for JSON configuration files
- **Testing**: Comprehensive unit tests with integration test compatibility

## Testing Guidelines

- **Unit tests**: Use `cargo test` for Rust-specific testing
- **Legacy compatibility**: Use `just test-legacy` to ensure compatibility with existing bats tests
- **Integration**: Maintain compatibility with shell script test suite during transition
- **Error conditions**: Test all error paths thoroughly
- **Cross-platform**: Ensure tests work on different operating systems
- **Test coverage**: Each implemented feature should have a test case (ideally written first)

### Test Utilities for Git-Based Testing

**Important**: For any tests that involve git operations or need isolated environments, use the common test utilities in `src/test_utils.rs`:

```rust
use crate::test_utils::test_helpers::*;
use tempfile::TempDir;

#[test]
fn test_with_git_environment() {
    let git_temp = TempDir::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
    let (_git_temp, git_service) = setup_test_repo();
    
    let mut config = create_test_config();
    config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
    
    // Your test code here using the isolated git environment
}
```

**Key Benefits:**
- **Test Independence**: Each test gets its own isolated git repository and state directory
- **Environment Isolation**: Tests don't interfere with each other or the host system
- **Automatic Cleanup**: The `TestEnvironmentGuard` automatically restores the original environment
- **Consistent Setup**: All git tests use the same standardized repository setup

**Available Helper Functions:**
- `setup_test_repo()`: Creates an isolated git repository with initial commit
- `create_test_config()`: Creates a clean test configuration
- `TestEnvironmentGuard::new()`: Sets up isolated environment and automatically restores on drop
- `setup_isolated_test_environment()`: Creates isolated config and state directories

**When to Use:**
- Any test that involves git operations (commits, branches, worktrees)
- Tests that use SessionManager or other components that modify file system state
- Integration tests that need multiple components working together
- Tests that need to avoid conflicts with other tests running in parallel

## Development Workflow

### Rust-First Development
1. Implement new features in Rust with proper error handling
2. Ensure CLI compatibility with original shell implementation
3. Run both Rust unit tests and legacy bats tests
4. Maintain feature parity during transition period

### Binary Compatibility
- The Rust binary should be a drop-in replacement for `para.sh`
- All existing command-line interfaces must be preserved
- Configuration files should remain compatible
- Session management must work with existing `.para_state/` directories

### Migration Strategy
- Rust implementation runs alongside shell version during development
- Use `just test-legacy` to verify compatibility with existing test suite
- Gradual replacement of shell components with Rust equivalents
- Maintain backward compatibility throughout transition

## Configuration System

### Cross-Platform Config
- Uses `directories` crate for platform-appropriate config paths
- JSON-based configuration with serde serialization
- Interactive wizard using `dialoguer` for setup
- Validation layer to ensure config integrity

### Configuration File Locations
The Rust implementation stores config in platform-specific directories:
- **macOS:** `~/Library/Application Support/para-rs/config.json`
- **Linux/Unix:** `~/.config/para-rs/config.json`
- **Windows:** `%APPDATA%\para-rs\config.json`

Configuration format (JSON):
```json
{
  "ide": {
    "name": "claude",
    "command": "claude",
    "user_data_dir": null,
    "wrapper": {
      "enabled": true,
      "name": "cursor",
      "command": "cursor"
    }
  },
  "directories": {
    "subtrees_dir": "subtrees/pc",
    "state_dir": ".para_state"
  },
  "git": {
    "branch_prefix": "pc",
    "auto_stage": true,
    "auto_commit": true
  },
  "session": {
    "default_name_format": "%Y%m%d-%H%M%S",
    "preserve_on_finish": true,
    "auto_cleanup_days": 30
  }
}
```

**Important for dispatch command:** Only Claude Code (standalone or wrapper mode) is supported.

### IDE Integration
- Same IDE support as shell version (Claude Code, Cursor, VS Code)
- Environment detection and configuration
- Cross-platform process launching

## Error Handling

- Use `anyhow::Result` for most application code
- Create specific error types with `thiserror` when needed
- Provide user-friendly error messages with context
- Maintain error compatibility with shell version expectations
