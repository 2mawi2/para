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
just setup-hooks      # Configure git pre-commit/pre-push hooks
just status           # Check dependencies and project health
just clean            # Clean development artifacts

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
- **Multi-Instance Support**: `para dispatch-multi` for parallel AI development
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

## Testing Guidelines

- Use `mktemp -d` for isolation in integration tests
- Test coverage includes units, integration, performance, and argument parsing
- All tests must pass before commits (enforced by git hooks)
- 99+ tests across 12 specialized test files ensure reliability