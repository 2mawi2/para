# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Para is a "Parallel IDE Workflow Helper" - a Rust implementation that enables developers to work on multiple features simultaneously using Git worktrees and their favorite IDEs. It's specifically designed for AI-assisted development workflows, allowing multiple Claude Code instances to work on different branches without conflicts.

## Essential Development Commands

### Setup and Testing
```bash
just test             # Run all checks (tests + linting + formatting) - ALWAYS use this
just test <filter>    # Run specific tests only (skips linting/formatting for speed)
                     # Examples: just test finish, just test integration, just test core::git
just lint             # Run clippy linting for all files
just fmt              # Auto-fix Rust formatting
just build            # Build debug binary (with bun/npm fallback for MCP server)
just build-release    # Build optimized release binary
```

### Common Development Tasks
```bash
just install          # Install Rust para binary globally
just uninstall        # Remove para from system
just setup-hooks      # Configure git pre-commit/pre-push hooks
just status           # Check dependencies and project health
just clean            # Clean development artifacts
just release          # Create new release (patch version bump)
```

**Testing Guidelines:**
- **Always use `just test`** for final verification (includes linting + formatting)
- Use `just test <filter>` during development for faster iteration on specific tests
- Use `just lint` to check linting on all modules
- The `just` commands have clean, focused output - avoid raw `cargo` commands

### TypeScript/MCP Testing
```bash
just test-ts          # Run TypeScript tests for MCP server
just lint-ts          # Run ESLint on TypeScript code
just fmt-ts           # Format TypeScript code
```

## Architecture Overview

### Modular Rust Design
```
src/
├── cli/              # Command-line interface, argument parsing, and command implementations
│   └── commands/     # Individual command implementations (start, finish, resume, etc.)
│       ├── mcp/      # MCP server management (init, strategies for finding servers)
│       ├── list/     # Session listing with formatters and analyzers
│       └── resume/   # Session resumption with context and task transformation
├── config/           # Configuration management, validation, and interactive wizard
├── core/             # Core business logic, session management, and IDE integration
│   ├── git/          # Git operations, worktree management, and repository handling
│   │   └── diff.rs   # Git diff calculation and statistics
│   └── status.rs     # Session status tracking and reporting
├── ui/               # Terminal UI components
│   └── monitor/      # Interactive session monitoring UI
│       ├── coordinator.rs    # Main UI coordination
│       ├── renderer.rs       # UI rendering logic
│       ├── state_manager.rs  # State management
│       └── actions.rs        # UI action handling
├── utils/            # Utility functions, error handling, and helper modules
│   └── validation.rs # Session name validation
└── main.rs           # Application entry point
```

### TypeScript MCP Server
```
mcp-server-ts/        # Model Context Protocol server implementation
├── src/
│   ├── para-mcp-server.ts      # Main MCP server implementation
│   └── para-mcp-server.test.ts # MCP server tests
├── package.json      # Node.js dependencies
├── tsconfig.json     # TypeScript configuration
└── eslint.config.js  # ESLint configuration
```

### Key Dependencies
- **clap**: Command-line argument parsing with completion support
- **serde/serde_json**: Configuration serialization
- **dialoguer**: Interactive prompts and wizards
- **chrono**: Date/time handling for sessions
- **anyhow/thiserror**: Error handling
- **directories**: Cross-platform config directories
- **ratatui**: Terminal UI framework for monitor command
- **crossterm**: Cross-platform terminal manipulation

### Key Features
- **Context-Aware**: Auto-detects current session from working directory
- **Auto-Staging**: Automatically stages all changes during finish operations
- **Recovery System**: Session snapshots for later recovery with `para recover`
- **File Input Support**: `para start --file prompt.txt` for complex prompts from files
- **IDE Wrapper Mode**: Claude Code can run inside VS Code/Cursor terminals
- **Interactive Monitor**: Real-time session monitoring with `para monitor` command
- **Status Tracking**: Update and view session status for AI agents with `para status`
- **MCP Integration**: Built-in TypeScript MCP server for Claude Code integration
- **Custom Branch Names**: Support for `--branch` flag in finish command

## Para Workflow Preferences

**My preferences for this project:**
- Use `para finish` workflow - create branches for manual review before merging
- Dispatched agents should use `para finish "<commit message>"` to create branches for review
- If you review multiple para branches (that have been finished) against their original TASK or intend, use your internal Task tool in parallel for that. Use one parallel Task for each review, and ensure that each Task does not checkout the branches but only reviews the git diff itself.

**Note**: Para MCP tools contain comprehensive workflow documentation. Check tool descriptions for parallel development patterns.

## Available Commands

### Core Commands
- `para start` - **UNIFIED COMMAND**: Create or resume para sessions
  - No args: Create new interactive session
  - With name only: Resume if exists, else create new
  - With prompt flag: Create new session with AI agent
  - With file: Read prompt/context from file
  - Examples:
    - `para start` - New interactive session
    - `para start feature-x` - Resume or create session
    - `para start -p "implement auth"` - New AI agent session
    - `para start --prompt "implement auth"` - New AI agent session (long form)
    - `para start feature-x -p "add tests"` - Named session with AI prompt
    - `para start --file tasks/auth.md` - From file
- `para finish "message" [--branch name]` - Complete session with commit
- `para list` - Show all active sessions
- `para cancel [session] [--force]` - Discard session
- `para clean` - Remove all sessions
- `para recover [session]` - Recover cancelled session
- `para monitor` - Interactive TUI for monitoring sessions
- `para status` - Update/view session status (for AI agents)

### Command Changes
- `para dispatch` - **REMOVED** - Use: `para start [name] -p "prompt"` for AI-assisted sessions
- `para start` - Now handles both interactive and AI-assisted session creation (use `-p/--prompt` flag for AI sessions)
- `para resume` - **Separate command** for continuing existing sessions

### MCP Commands
- `para mcp init` - Set up MCP integration in current repo
- `para mcp` - Manage MCP server configuration

## File Structure Notes

### Important Locations
- **Config**: Platform-specific config directories (see Configuration System above)
- **State**: `.para/state/` directory for session tracking
- **Documentation**: `docs/` directory with detailed guides

### Development Files
- **`justfile`**: All development automation commands
- **`Cargo.toml`**: Rust package configuration
- **`scripts/pre-commit`** and **`scripts/pre-push`**: Git hooks for quality control
- **`.github/workflows/`**: GitHub Actions for CI/CD and automated reviews
- **`mcp-server-ts/`**: TypeScript MCP server implementation

## Code Style Requirements

- **Error handling**: Use `anyhow::Result` for application errors, `thiserror` for custom error types
- **CLI design**: Follow clap conventions with derive API
- **Configuration**: Use serde for JSON configuration files
- **Context-aware operations**: Detect session state from working directory
- **Comprehensive error handling** with user-friendly messages
- **Auto-staging workflow**: Users should never need manual `git add`
- **Dead Code**: Never allow dead code with `#[allow(dead_code)]`
- **Unused Imports**: Never allow unused imports with `#[allow(unused_imports)]`
- **Disabled Tests**: Never disable tests with `#[cfg(disabled_test)]`

## Testing Guidelines

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
- `create_test_config()`: Creates a clean test configuration with mock IDE commands
- `create_test_config_with_dir()`: Creates test config with custom state directory
- `TestEnvironmentGuard::new()`: Sets up isolated environment and automatically restores on drop
- `setup_isolated_test_environment()`: Creates isolated config and state directories

**Important for Test Setup:**
- Always pre-create `.para` and state directories in test repos to avoid race conditions
- Use the test utilities instead of creating custom test setups
- Tests run in parallel, so ensure proper isolation using the provided utilities

**When to Use:**
- Any test that involves git operations (commits, branches, worktrees)
- Tests that use SessionManager or other components that modify file system state
- Integration tests that need multiple components working together
- Tests that need to avoid conflicts with other tests running in parallel

### Testing Requirements
- **Unit tests**: Use `cargo test` for Rust-specific testing
- **Error conditions**: Test all error paths thoroughly
- **Cross-platform**: Ensure tests work on different operating systems
- **Test coverage**: Each implemented feature should have a test case (ideally written first)
- **TDD**: Write tests before implementing features, try to make them red first, then implement the feature to make them green.
- **UI/UX**: Try to make the tests independent of UI/UX and hardware. If there is cyclomatic complexity, try to isolate the business logic into seperate components and test them there.
- **Execute tests before summary**: Before the final summary, you should almost always execute the tests. This shows that you have tested the code, and you are not reward hacking in the final summary.
- **Sugar Talk**: Never sugar talk your final summary, be honest about red tests or missing implementation details.
- All tests must pass before commits (enforced by git hooks)
- There is no such thing as a 'minor test issue' - if a test fails, it's a bug and should be fixed immediately
- No task is ever done if not all tests ('just test') pass, every other reward hacking is a ethically wrong lie to the user!
- Ensure no lint issues exist, if they do, fix them. (Otherwise the pipeline will fail)
- Tests are never 'partially green' - only if `just test` passes, the definition of green is met.

### Test Isolation Requirements
**CRITICAL**: Tests must NEVER interact with the user's real configuration or system:
- **Config Isolation**: The `TestEnvironmentGuard` automatically sets `PARA_CONFIG_PATH` to a test-specific config with mock IDE commands
- **Mock IDE Commands**: Test configs always use `"echo"` as the IDE command - NEVER real commands like `"cursor"`, `"claude"`, or `"code"`
- **Environment Safety**: Tests that call `execute()` functions will use the isolated test config, preventing them from launching or killing real IDEs
- **No Host Interaction**: Tests must not access:
  - Real user config at `~/Library/Application Support/para/config.json`
  - Real IDE processes
  - User's home directory settings
  - Any system-wide para installations
- **Verification**: Run `just test test_safety` to verify test isolation is working correctly

## Code Reviews

- Stay critical, focus on functionality, not style.
- **Review for correctness and safety**: Always check for logic errors, edge cases, and potential panics or unsafe operations.
- **Check for clear, actionable error messages**: Ensure all errors provide enough context for debugging and user understanding.
- **Assess modularity and testability**: Code should be broken into small, testable units with clear responsibilities.
- **Verify adherence to project conventions**: Confirm code follows established patterns for configuration, error handling, and CLI design.

## Development Preferences


### Release Process
- Use `just release` to create new releases (automatically bumps patch version and creates GitHub release)
- Must be on main branch with clean working directory
- Requires GitHub Actions workflow for automated publishing
- Github releases can be watched with `gh run watch <release-id> --exit-status`, this allows to check if a release succeeded or failed.

### Commit Style  
- Keep commit messages short and concise (one line preferred)
- No Claude attributions or co-authored tags
- Focus on what changed, not implementation details

### Error Handling
- Use `anyhow::Result` for most application code
- Create specific error types with `thiserror` when needed
- Provide user-friendly error messages with context
- Always provide specific, user-friendly error messages
- Include relevant context (file names, session names, etc.) in error messages
- Test error conditions thoroughly with unit tests


# Thinking

- If you are stuck on a problem, or you iterated on fixing a test for multiple iterations:
1. Think deeply about the problem first, and find the best option that solves the issue.
2. If this still doesn't help: Use the user for help, but do this very rarely.
3. Never try using dirty fixes and workarounds for a hard problem, solve it directly by thinking deep or asking the user for help.


# Git

- Before running git commands that are branch related, ensure to run 'git status' to check the branch you are on to avoid executing operations on the wrong branch.