# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Para is a "Parallel IDE Workflow Helper" - a POSIX shell script that enables developers to work on multiple features simultaneously using Git worktrees and their favorite IDEs. It's specifically designed for AI-assisted development workflows, allowing multiple Claude Code instances to work on different branches without conflicts.

## Essential Development Commands

### Setup and Testing
```bash
just dev-setup        # Complete development environment setup (installs bats, shellcheck, shfmt)
just test             # Run all 99+ tests across 12 test suites
just test-unit        # Run unit tests only
just test-integration # Run integration tests only
just lint             # Run shellcheck + shfmt linting
just fmt              # Auto-fix shell script formatting
just benchmark        # Run performance benchmarks
```

### Common Development Tasks
```bash
just install          # Install para globally
just uninstall        # Remove para from system
just setup-hooks      # Configure git pre-commit/pre-push hooks
just status           # Check dependencies and project health
just clean            # Clean development artifacts
just release          # Create new release (patch version bump)

# Testing specific components
just test tests/test_para_units.bats              # Run specific test file
just test-prompts                                 # Test prompt features
just test-args                                    # Test argument parsing
```

### Testing Requirements
- **Framework**: bats-core for shell script testing
- **Important**: When writing integration tests, always use `mktemp -d` to ensure git commands don't interfere with the main repository
- **Coverage**: 12 specialized test files covering units, integration, performance, argument parsing, prompt features, and more

## Architecture Overview

### Modular Library Design
```
lib/
├── para-commands.sh       # Command implementations (dispatch, start, finish, etc.)
├── para-config.sh         # Configuration management and environment setup
├── para-config-wizard.sh  # Interactive configuration wizard  
├── para-session.sh        # Session lifecycle and state management
├── para-git.sh            # Git operations and worktree management
├── para-ide.sh            # IDE integration (Cursor, Claude Code, VS Code)
└── para-utils.sh          # Utility functions and validation
```

### Core Workflow
1. **Entry Point** (`para.sh`) loads all library modules and dispatches commands
2. **Configuration System** (`para-config.sh`) manages IDE preferences and environment
3. **Session Management** (`para-session.sh`) creates isolated development environments
4. **Git Operations** (`para-git.sh`) handles worktree creation and branch management
5. **IDE Integration** (`para-ide.sh`) launches configured editors with session context

### Key Features
- **Context-Aware**: Auto-detects current session from working directory
- **Auto-Staging**: Automatically stages all changes during finish operations
- **Recovery System**: Session snapshots for later recovery with `para recover`
- **File Input Support**: `para dispatch --file prompt.txt` for complex prompts from files
- **IDE Wrapper Mode**: Claude Code can run inside VS Code/Cursor terminals

## Configuration System

### IDE Support Hierarchy
1. **Claude Code** (Recommended for AI development)
   - Can run standalone or inside other IDEs (wrapper mode)
   - Auto-detects terminal for standalone mode
   - Environment variables: `IDE_NAME="claude"`, `IDE_CMD="claude"`

2. **Cursor** (Default)
   - Environment variables: `IDE_NAME="cursor"`, `IDE_CMD="cursor"`

3. **VS Code**
   - Environment variables: `IDE_NAME="code"`, `IDE_CMD="code"`

### Configuration Commands
```bash
para config        # Interactive configuration wizard
para config auto   # Auto-detect and configure IDE
para config show   # Display current configuration
para config edit   # Open config file in editor
```

## Session Management Pattern

Para creates timestamped sessions with complete isolation:
- **Branch**: `pc/YYYYMMDD-HHMMSS` format
- **Worktree**: Created in `subtrees/pc/` directory  
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
- This will bring the changes to a new branch with the agent's name. This branch needs to be integrated in the master branch, and all conflicts need to be resolved. After this is done, tests have to be run again to ensure that the integration didn't break anything 
- No not use you internal Agent system to process the task. Call `para dispatch` to start a new agent for the task.

**Writing Tasks**:
- Write tasks in the root directory in the format of `TASK_<number>_<description>.md`
- Keep tasks concise but include all the information needed to complete the task.
- Always include at the end the instructions for the agent to run the tests and linters and review the changes.
- Ensure that the task is written is such a way that it can be completed in the git subtree on it's own.
- Always clearly specify that the agent must call `para finish '<commit message>'` to finish the task. (this is a must for the agent)

## File Structure Notes

### Important Locations
- **Config**: `${XDG_CONFIG_HOME:-$HOME/.config}/para/config`
- **State**: `.para_state/` directory for session tracking
- **Tests**: `tests/` directory with comprehensive bats test suites
- **Documentation**: `docs/` directory with detailed guides

### Development Files
- **`justfile`**: All development automation commands
- **`install-para.sh`**: Universal installer with Homebrew support
- **`scripts/pre-commit`** and **`scripts/pre-push`**: Git hooks for quality control

## Code Style Requirements

- **Pure POSIX shell** for maximum compatibility
- **No bash-specific features** - must work on various shells
- **Modular design** - keep functionality in appropriate lib/ files
- **Context-aware operations** - detect session state from working directory
- **Comprehensive error handling** with user-friendly messages
- **Auto-staging workflow** - users should never need manual `git add`
- **No comments or docstrings** - try to avoid comments completely and use self-documenting code (if possible)
- **Dead Code** - never allow dead code with `#[allow(dead_code)]`
- **Unused Imports** - never allow unused imports with `#[allow(unused_imports)]`
- **Disabled Tests** - never disable tests with `#[cfg(disabled_test)]`

## Testing Guidelines

- Use `mktemp -d` for isolation in integration tests
- Test coverage includes units, integration, performance, and argument parsing
- All tests must pass before commits (enforced by git hooks)
- 99+ tests across 12 specialized test files ensure reliability
- There is no such thing as a 'minor test issue' - if a test fails, it's a bug and should be fixed immediately
- No task is ever done if not all tests ('just test') pass, every other reward hacking is a ethically wrong lie to the user!
- Ensure no lint issues exist, if they do, fix them. (Otherwise the pipeline will fail)

## Development Preferences

### Para Rules
- You never run `para cancel` yourself, this will likely delete the session you are working on and all of it's data.

### Release Process
- Use `just release` to create new releases (automatically bumps patch version and creates GitHub release)
- Must be on master branch with clean working directory
- Requires GitHub Actions workflow for automated publishing

### Commit Style  
- Keep commit messages short and concise (one line preferred)
- No Claude attributions or co-authored tags
- Focus on what changed, not implementation details

### Error Handling
- Always provide specific, user-friendly error messages
- Include relevant context (file names, session names, etc.) in error messages
- Test error conditions thoroughly with unit tests