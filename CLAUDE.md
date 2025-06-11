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
just build            # Build debug binary
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

### Key Features
- **Context-Aware**: Auto-detects current session from working directory
- **Auto-Staging**: Automatically stages all changes during finish operations
- **Recovery System**: Session snapshots for later recovery with `para recover`
- **File Input Support**: `para dispatch --file prompt.txt` for complex prompts from files
- **IDE Wrapper Mode**: Claude Code can run inside VS Code/Cursor terminals

## Configuration System

### Cross-Platform Config
- Uses `directories` crate for platform-appropriate config paths
- JSON-based configuration with serde serialization
- Interactive wizard using `dialoguer` for setup
- Validation layer to ensure config integrity

### Configuration File Locations
The Rust implementation stores config in platform-specific directories:
- **macOS:** `~/Library/Application Support/para/config.json`
- **Linux/Unix:** `~/.config/para/config.json`
- **Windows:** `%APPDATA%\para\config.json`

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
    "subtrees_dir": "subtrees/para",
    "state_dir": ".para_state"
  },
  "git": {
    "branch_prefix": "para",
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

### IDE Support Hierarchy
1. **Claude Code** (Recommended for AI development)
   - Can run standalone or inside other IDEs (wrapper mode)
   - Auto-detects terminal for standalone mode

2. **Cursor** (Default)
   - Environment variables supported

3. **VS Code**
   - Environment variables supported

### Configuration Commands
```bash
para config        # Interactive configuration wizard
para config auto   # Auto-detect and configure IDE
para config show   # Display current configuration
para config edit   # Open config file in editor
```

## Session Management Pattern

Para creates timestamped sessions with complete isolation:
- **Branch**: `para/YYYYMMDD-HHMMSS` format
- **Worktree**: Created in `subtrees/para/` directory  
- **State**: Tracked in `.para_state/` directory
- **Recovery**: Sessions preserved for later recovery

### Typical Workflow
```bash
para start feature-auth           # Creates worktree + branch + opens IDE
# Work in the session...
para finish "Implement OAuth"     # Auto-stages, commits, switches back
para recover feature-auth         # Restore if needed later
```

### AI-Powered Development with Claude Code
```bash
# Single session with inline prompt
para dispatch "Implement user authentication system"

# Single session with prompt from file
para dispatch --file auth-requirements.prompt
para dispatch -f ./complex-task.txt

# Named session with file input
para dispatch auth-feature --file auth-spec.md
```

### Parallel Development with Multiple Agents
Para supports dispatching multiple agents to work on different tasks simultaneously:

```bash
# Dispatch agents with task files for parallel development
para dispatch task1-agent --file TASK_1_IMPLEMENTATION.md -d
para dispatch task2-agent --file TASK_2_IMPLEMENTATION.md -d
para dispatch task3-agent --file TASK_3_IMPLEMENTATION.md -d
```

**Key Concepts**:
- Each agent gets an isolated worktree and branch
- Task specifications should be comprehensive markdown files
- Use `--file` flag to pass complete task requirements to agents
- Add `-d` to prevent IDE permission prompts during automation
- Always prompt the agent at the end to ensure all tests are green, All linters are green, and the code is reviewed by executing a git diff on its own changes and then reviewing what it has done 
- Try avoid conflicts in between agents. No agent should have a task that depends on another task agent being run at the same time.
- If one task requires another task, Start those tasks only sequentially. Once the first task is integrated we can start the second one and so on. 
- When an agent is ready it should call para finish '<commit message>' to finish the task.
- This will bring the changes to a new branch with the agent's name. This branch needs to be integrated in the main branch, and all conflicts need to be resolved. After this is done, tests have to be run again to ensure that the integration didn't break anything 
- No not use you internal Agent system to process the task. Call `para dispatch` to start a new agent for the task.

**Writing Tasks**:
- Write tasks in the root directory in the format of `TASK_<number>_<description>.md`
- Keep tasks concise but include all the information needed to complete the task.
- Always include at the end the instructions for the agent to run the tests and linters and review the changes.
- Ensure that the task is written is such a way that it can be completed in the git subtree on it's own.
- Always clearly specify that the agent must call `para finish '<commit message>'` to finish the task. (this is a must for the agent)
- Never overengineer tasks, try to keep the task as simple as possible, while balancing the requirements of the user.

## File Structure Notes

### Important Locations
- **Config**: Platform-specific config directories (see Configuration System above)
- **State**: `.para_state/` directory for session tracking
- **Documentation**: `docs/` directory with detailed guides

### Development Files
- **`justfile`**: All development automation commands
- **`Cargo.toml`**: Rust package configuration
- **`scripts/pre-commit`** and **`scripts/pre-push`**: Git hooks for quality control

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
- `create_test_config()`: Creates a clean test configuration
- `TestEnvironmentGuard::new()`: Sets up isolated environment and automatically restores on drop
- `setup_isolated_test_environment()`: Creates isolated config and state directories

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

## Code Reviews

- Stay critical, focus on functionality, not style.
- **Review for correctness and safety**: Always check for logic errors, edge cases, and potential panics or unsafe operations.
- **Check for clear, actionable error messages**: Ensure all errors provide enough context for debugging and user understanding.
- **Assess modularity and testability**: Code should be broken into small, testable units with clear responsibilities.
- **Verify adherence to project conventions**: Confirm code follows established patterns for configuration, error handling, and CLI design.

## Development Preferences

### Para Rules
- You never run `para cancel` yourself, this will likely delete the session you are working on and all of it's data.

### Release Process
- Use `just release` to create new releases (automatically bumps patch version and creates GitHub release)
- Must be on main branch with clean working directory
- Requires GitHub Actions workflow for automated publishing

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