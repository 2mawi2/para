# Pursor - Parallel Cursor Workflow Helper

A modular POSIX shell script for creating multiple ephemeral Cursor IDE sessions on temporary Git worktrees, enabling parallel development with easy merge/discard workflow.

Perfect for prototyping multiple features simultaneously while keeping your main branch clean.

## 🚀 Quick Start

```bash
# Create a new parallel session
pursor

# Work in the new Cursor window that opens...

# Merge your changes back
pursor merge "Add new feature"
```

## 🛠️ Development

### Setup Development Environment

```bash
# Install Just command runner (if not already installed)
brew install just  # macOS
# or curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to ~/bin

# Complete development setup
just dev-setup
```

### Available Commands

```bash
just install        # Install pursor globally
just run [ARGS]     # Run local pursor.sh with arguments
just test           # Run bats test suite (auto-installs dependencies)
just lint           # Run shellcheck + shfmt linting
just fmt            # Auto-fix shell script formatting
just setup-hooks    # Configure git pre-commit/pre-push hooks
just status         # Show project status and dependencies
just clean          # Clean up development artifacts
```

### Git Hooks (Auto-configured)
- **Pre-commit**: Runs tests before commits
- **Pre-push**: Runs linting before pushes

### Testing
- Uses [bats-core](https://github.com/bats-core/bats-core) for shell script testing
- Tests in `tests/test_pursor.bats`
- Run with `just test` or `bats tests/`

### Linting
- **shellcheck** for static analysis
- **shfmt** for consistent formatting
- Run with `just lint` or individually

## 🏗️ Architecture

Pursor is built with a modular architecture for maintainability and extensibility:

```
pursor/
├── pursor.sh              # Main entry point and command dispatch
├── justfile               # Development workflow automation
├── scripts/               # Git hook templates
└── lib/                   # Modular library components
    ├── pursor-config.sh   # Configuration management
    ├── pursor-utils.sh    # Utility functions and validation
    ├── pursor-git.sh      # Git operations and worktree management
    ├── pursor-session.sh  # Session lifecycle and state management
    └── pursor-ide.sh      # IDE integration (extensible for future IDEs)
```

This design enables:
- **Clean separation of concerns** - each module handles specific functionality
- **Easy extensibility** - add support for new IDEs or features by extending modules
- **Better maintainability** - isolated, testable components
- **Reliable installation** - automatic module discovery and installation

## 📦 Installation

### Universal Installer (Recommended)

The universal installer works with **any shell** (bash, zsh, fish, sh, etc.) and automatically handles the modular structure:

```bash
# Download and run the universal installer
./install-pursor.sh
```

This will:
- Auto-detect your shell (bash/zsh/fish/other)
- Install the complete modular structure to `~/.local/lib/pursor/`
- Create a wrapper command at `~/.local/bin/pursor`
- Add `~/.local/bin` to your PATH in the appropriate config file
- Work immediately after installation

### Manual Installation

1. Copy the entire directory structure (including `lib/`) to your desired location
2. Make the main script executable: `chmod +x pursor.sh`
3. Optionally, copy to `~/.local/lib/pursor/` and create a wrapper for global access

**Shell Compatibility**: Pursor is written in POSIX shell and works with all major shells including bash, zsh, fish, dash, and ash.

## 🎯 Usage

### Basic Commands

```bash
pursor                    # Create new session → opens Cursor
pursor list               # List all active sessions (alias: ls)
pursor merge "message"    # Merge session back to main
pursor continue           # Continue merge after resolving conflicts
pursor cancel             # Cancel/delete session (alias: abort)
pursor clean              # Cancel ALL sessions (clean everything)
pursor resume <session>   # Resume/reconnect to existing session
```

### Named Sessions

```bash
# Create named sessions for better organization
pursor feature-auth       # Creates session "feature-auth"
pursor bugfix-login       # Creates session "bugfix-login"

# List shows friendly names
pursor list
# Session: feature-auth
#   Branch: pc/feature-auth-20250531-143022
#   Status: Has uncommitted changes
#   Resume: pursor resume feature-auth

# Resume specific sessions
pursor resume feature-auth
```

### Multi-Session Workflow

```bash
# Create multiple sessions
pursor                    # Session 1 (opens Cursor)
pursor feature-auth       # Named session (opens Cursor) 

# List active sessions
pursor list

# Merge sessions (auto-detects from current directory!)
cd subtrees/pc/20250531-143022
pursor merge "Feature A complete"

cd ../feature-auth-20250531-143025
pursor merge "Authentication complete"

# Or cancel individual sessions
pursor cancel feature-auth

# Or cancel ALL sessions at once
pursor clean
```

### Quick Reset

When you want to start fresh and clean up all parallel sessions:

```bash
pursor clean              # Cleans up everything
pursor list               # Verify: "No active parallel sessions."
```

## 🔧 Handling Conflicts

When merging sessions that modify the same files, you might get conflicts:

```bash
# Try to merge
pursor merge "Add feature"
# ❌ rebase conflicts
#    → resolve conflicts in /path/to/worktree
#    → then run: pursor continue

# Fix conflicts manually in the worktree directory
cd subtrees/pc/20250531-143022
# Edit conflicted files to resolve conflicts
# (NO need to run git add!)

# Continue the merge with auto-staging
pursor continue
# ✅ merge complete!
```

## 📂 How It Works

- **Session Creation**: Creates timestamped branch `pc/YYYYMMDD-HHMMSS` and worktree in `subtrees/`
- **State Tracking**: Uses `.pursor_state/` directory to track sessions
- **Context-Aware**: Auto-detects current session from working directory
- **Auto-Staging**: Automatically stages all changes during merge and conflict resolution
- **Clean Workflow**: No manual `git add` required anywhere

## 🔧 Configuration

Pursor can be configured via environment variables:

```bash
# Override default settings
export BASE_BRANCH="develop"                 # Use different base branch
export SUBTREES_DIR_NAME="worktrees"         # Change worktree directory name
export STATE_DIR_NAME=".pursor"              # Change state directory name
export CURSOR_CMD="code"                     # Use different editor command
export CURSOR_USER_DATA_DIR=".cursor-userdata"  # Isolated user data directory for each session
```

### Cursor Isolation Feature

By default, Pursor launches each session with an isolated user data directory (`.cursor-userdata` within each worktree). This provides complete separation from your main Cursor workspace.

**Benefits:**
- **Complete isolation**: Parallel sessions have their own recent files, settings, and extensions
- **No clutter**: Your main Cursor "recent projects" list stays clean
- **Fresh start**: Each session starts with a clean environment
- **Easy cleanup**: Session data is automatically removed when sessions are cleaned up

**Default behavior:**
```bash
pursor                    # Uses isolated user data directory automatically
```

**Custom user data directory name:**
```bash
export CURSOR_USER_DATA_DIR=".my-cursor-data"
pursor                    # Uses custom directory name
```

**Disable isolation (use main Cursor instance):**
```bash
unset CURSOR_USER_DATA_DIR
pursor                    # Uses your main Cursor instance
```

**How it works:**
- Each worktree gets its own `.cursor-userdata/` directory
- Cursor launches with `--user-data-dir` pointing to this isolated directory
- Settings, extensions, and recent files are completely separate per session
- When you clean up sessions, the isolated data is removed too

## 🚀 Extensibility

The modular architecture makes it easy to extend pursor:

### Adding IDE Support

To add support for a new IDE, extend `lib/pursor-ide.sh`:

```bash
# Add new IDE implementation
launch_vscode() {
  worktree_dir="$1"
  if command -v code >/dev/null 2>&1; then
    echo "▶ launching VS Code..."
    code "$worktree_dir" &
    echo "✅ VS Code opened"
  else
    echo "⚠️  VS Code not found"
  fi
}

# Update the main launch_ide function
launch_ide() {
  ide_name="$1"
  worktree_dir="$2"
  
  case "$ide_name" in
    cursor) launch_cursor "$worktree_dir" ;;
    vscode) launch_vscode "$worktree_dir" ;;
    *) die "unsupported IDE: $ide_name" ;;
  esac
}
```

### Adding New Commands

Add new commands by extending the `handle_command` function in `pursor.sh` and implementing the logic in appropriate modules.

## 🧪 Development

### Testing Workflows

**Basic Test:**
```bash
./pursor.sh                          # Create session
cd subtrees/pc/*/                    # Enter worktree
echo 'test change' >> test-file.py   # Make changes
./pursor.sh merge "test commit"      # Auto-stage & merge
```

**Conflict Test:**
```bash
./pursor.sh && ./pursor.sh           # Create 2 sessions
cd subtrees/pc/20*/                  # Session 1: modify same file
echo 'change A' >> test-file.py && ../../../pursor.sh merge "A"
cd ../20*/                           # Session 2: conflicting change
echo 'change B' >> test-file.py && ../../../pursor.sh merge "B"  # Conflict!
# Edit file to resolve conflicts, then:
./pursor.sh continue                 # Auto-stages & completes
```

### Module Structure

Each module in `lib/` has a specific responsibility:

- **pursor-config.sh**: Environment setup and configuration loading
- **pursor-utils.sh**: Common utilities, validation, and helper functions
- **pursor-git.sh**: All Git operations including worktree and merge management
- **pursor-session.sh**: Session lifecycle, state management, and detection
- **pursor-ide.sh**: IDE integration with extensible interface

## 🤝 Contributing

The modular architecture makes contributions easier:

1. **Bug fixes**: Usually isolated to a single module
2. **New features**: Can often be added by extending existing modules
3. **IDE support**: Add new implementations to `pursor-ide.sh`
4. **Git workflows**: Extend `pursor-git.sh` for new merge strategies

## 📋 Requirements

- **Git** with worktree support (Git 2.5+)
- **POSIX shell** (bash, zsh, fish, dash, ash, etc.)
- **Cursor IDE** (or configure `CURSOR_CMD` for different editor)

## 🐛 Troubleshooting

### Common Issues

**"not in a Git repository"**
- Run pursor from within a Git repository

**"Cursor CLI not found"**
- Install Cursor CLI or set `CURSOR_CMD` environment variable

**"session not found"**
- Use `pursor list` to see active sessions
- Ensure you're in the correct directory for auto-detection

### Debug Mode

For debugging, you can trace execution:
```bash
set -x  # Enable shell tracing
./pursor.sh your-command
set +x  # Disable tracing
```