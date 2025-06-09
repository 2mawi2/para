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

### IDE Integration
- Same IDE support as shell version (Claude Code, Cursor, VS Code)
- Environment detection and configuration
- Cross-platform process launching

## Error Handling

- Use `anyhow::Result` for most application code
- Create specific error types with `thiserror` when needed
- Provide user-friendly error messages with context
- Maintain error compatibility with shell version expectations
