# Para - Parallel IDE Workflow Helper

A modular POSIX shell script for creating multiple ephemeral Cursor IDE sessions on temporary Git worktrees, enabling parallel development with easy finish/discard workflow.

Perfect for prototyping multiple features simultaneously while keeping your main branch clean.

**Supports multiple IDEs:** Cursor, Claude Code, VS Code, and extensible for others.

## 🚀 Quick Start

```bash
# Create a new parallel session (gets friendly name like "swift_phoenix_20250531-143022")
para start

# Work in the new IDE window that opens...

# Finish your changes back to main branch
para finish "Add new feature"
```

## ⚙️ IDE Configuration

Para supports multiple IDEs with easy configuration:

### Claude Code (Recommended for Claude AI users)
```bash
export IDE_NAME="claude"
export IDE_CMD="claude"
# Note: Claude Code doesn't support --user-data-dir isolation
para  # Now launches Claude Code instead of Cursor
```

### Cursor (Default)
```bash
export IDE_NAME="cursor"  # Default
export IDE_CMD="cursor"   # Default
export IDE_USER_DATA_DIR=".cursor-userdata"  # Default
```

### VS Code
```bash
export IDE_NAME="code"
export IDE_CMD="code"
export IDE_USER_DATA_DIR=".vscode-userdata"
```

### Custom IDE
```bash
export IDE_NAME="my-editor"
export IDE_CMD="my-editor-command"
export IDE_USER_DATA_DIR=".my-editor-userdata"
```

**Make configuration persistent:** Add these exports to your shell configuration file (`~/.bashrc`, `~/.zshrc`, `~/.config/fish/config.fish`, etc.)

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
just install        # Install para globally
just run [ARGS]     # Run local para.sh with arguments
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
- Tests in `tests/test_para.bats`
- Run with `just test` or `bats tests/`

### Linting
- **shellcheck** for static analysis
- **shfmt** for consistent formatting
- Run with `just lint` or individually

## 🏗️ Architecture

Para is built with a modular architecture for maintainability and extensibility:

```
para/
├── para.sh              # Main entry point and command dispatch
├── justfile               # Development workflow automation
├── scripts/               # Git hook templates
└── lib/                   # Modular library components
    ├── para-config.sh   # Configuration management
    ├── para-utils.sh    # Utility functions and validation
    ├── para-git.sh      # Git operations and worktree management
    ├── para-session.sh  # Session lifecycle and state management
    └── para-ide.sh      # IDE integration (extensible for future IDEs)
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
./install-para.sh
```

This will:
- Auto-detect your shell (bash/zsh/fish/other)
- Install the complete modular structure to `~/.local/lib/para/`
- Create a wrapper command at `~/.local/bin/para`
- Add `~/.local/bin` to your PATH in the appropriate config file
- Work immediately after installation

### Manual Installation

1. Copy the entire directory structure (including `lib/`) to your desired location
2. Make the main script executable: `chmod +x para.sh`
3. Optionally, copy to `~/.local/lib/para/` and create a wrapper for global access

**Shell Compatibility**: Para is written in POSIX shell and works with all major shells including bash, zsh, fish, dash, and ash.

## 🎯 Usage

### Session Naming

Para uses **friendly names** for auto-generated sessions, making them much easier to remember and type:

```bash
para start                  # Creates session with friendly name like "swift_phoenix_20250531-143022"
para start feature-auth     # Creates custom named session "feature-auth"

# Friendly names are:
# - Easy to remember: "swift_phoenix" vs "20250531-143022"
# - Easy to type: No long number sequences
# - Still unique: Timestamp suffix ensures uniqueness
# - Consistent: Same adjective/noun pattern as Docker Compose
```

### Basic Commands

```bash
para start                  # Create new session → opens Cursor
para list                   # List all active sessions (alias: ls)
para finish "message"       # Finish session back to main
para continue               # Continue finish after resolving conflicts
para cancel                 # Cancel/delete session (alias: abort)
para clean                  # Cancel ALL sessions (clean everything)
para resume <session>       # Resume/reconnect to existing session
```

### Named Sessions

```bash
# Create named sessions for better organization
para start feature-auth     # Creates session "feature-auth"
para start bugfix-login     # Creates session "bugfix-login"

# List shows friendly names
para list
# Session: feature-auth
#   Branch: pc/feature-auth-20250531-143022
#   Status: Has uncommitted changes
#   Resume: para resume feature-auth

# Resume specific sessions
para resume feature-auth
```

### Multi-Session Workflow

```bash
# Create multiple sessions
para start                  # Session 1 (opens Cursor)
para start feature-auth     # Named session (opens Cursor)

# List active sessions
para list

# Finish sessions (auto-detects from current directory!)
cd subtrees/pc/20250531-143022
para finish "Feature A complete"

cd ../feature-auth-20250531-143025
para finish "Authentication complete"

# Or cancel individual sessions
para cancel feature-auth

# Or cancel ALL sessions at once
para clean
```

### Quick Reset

When you want to start fresh and clean up all parallel sessions:

```bash
para clean              # Cleans up everything
para list               # Verify: "No active parallel sessions."
```

## 🔧 Handling Conflicts

When finishing sessions that modify the same files, you might get conflicts:

```bash
# Try to finish
para finish "Add feature"
# ❌ finish conflicts
#    → resolve conflicts in /path/to/worktree
#    → then run: para continue

# Fix conflicts manually in the worktree directory
cd subtrees/pc/20250531-143022
# Edit conflicted files to resolve conflicts
# (NO need to run git add!)

# Continue the finish with auto-staging
para continue
# ✅ finish complete!
```

## 📂 How It Works

- **Session Creation**: Creates timestamped branch `pc/YYYYMMDD-HHMMSS` and worktree in `subtrees/`
- **State Tracking**: Uses `.para_state/` directory to track sessions
- **Context-Aware**: Auto-detects current session from working directory
- **Auto-Staging**: Automatically stages all changes during finish and conflict resolution
- **Clean Workflow**: No manual `git add` required anywhere

## 🔧 Configuration

Para can be configured via environment variables:

```bash
# IDE Configuration
export IDE_NAME="claude"                     # IDE to use (claude, cursor, code, etc.)
export IDE_CMD="claude"                      # Command to launch IDE
export IDE_USER_DATA_DIR=".claude-userdata" # Isolated user data directory

# Other settings
export BASE_BRANCH="develop"                 # Use different base branch
export SUBTREES_DIR_NAME="worktrees"         # Change worktree directory name
export STATE_DIR_NAME=".para"              # Change state directory name

# Legacy compatibility (still supported)
export CURSOR_CMD="cursor"                   # Backwards compatible
export CURSOR_USER_DATA_DIR=".cursor-userdata"  # Backwards compatible
```

### IDE Isolation Feature

By default, Para launches each session with an isolated user data directory (e.g., `.claude-userdata` within each worktree). This provides complete separation from your main IDE workspace.

**Benefits:**
- **Complete isolation**: Parallel sessions have their own recent files, settings, and extensions
- **No clutter**: Your main IDE "recent projects" list stays clean
- **Fresh start**: Each session starts with a clean environment
- **Easy cleanup**: Session data is automatically removed when sessions are cleaned up

**Default behavior:**
```bash
para start                  # Uses isolated user data directory automatically
```

**Custom user data directory name:**
```bash
export CURSOR_USER_DATA_DIR=".my-cursor-data"
para start                  # Uses custom directory name
```

**Disable isolation (use main IDE instance):**
```bash
unset CURSOR_USER_DATA_DIR
para start                  # Uses your main Cursor instance
```

**How it works:**
- Each worktree gets its own user data directory (e.g., `.claude-userdata/`)
- IDE launches with `--user-data-dir` pointing to this isolated directory
- Settings, extensions, and recent files are completely separate per session
- When you clean up sessions, the isolated data is removed too

## 🚀 Extensibility

The modular architecture makes it easy to extend para:

### Adding IDE Support

To add support for a new IDE, extend `lib/para-ide.sh`:

```bash
# Add new IDE implementation
launch_neovim() {
  worktree_dir="$1"
  if command -v nvim >/dev/null 2>&1; then
    echo "▶ launching Neovim..."
    nvim "$worktree_dir" &
    echo "✅ Neovim opened"
  else
    echo "⚠️  Neovim not found"
  fi
}

# Update the main launch_ide function
launch_ide() {
  ide_name="$1"
  worktree_dir="$2"

  case "$ide_name" in
    cursor) launch_cursor "$worktree_dir" ;;
    claude) launch_claude "$worktree_dir" ;;
    code) launch_vscode "$worktree_dir" ;;
    neovim) launch_neovim "$worktree_dir" ;;
    *) die "unsupported IDE: $ide_name" ;;
  esac
}
```

### Adding New Commands

Add new commands by extending the `handle_command` function in `para.sh` and implementing the logic in appropriate modules.

## 🧪 Development

### Testing Workflows

**Basic Test:**
```bash
./para.sh start                     # Create session
cd subtrees/pc/*/                    # Enter worktree
echo 'test change' >> test-file.py   # Make changes
./para.sh finish "test commit"      # Auto-stage & finish
```

**Conflict Test:**
```bash
./para.sh start && ./para.sh start  # Create 2 sessions
cd subtrees/pc/20*/                  # Session 1: modify same file
echo 'change A' >> test-file.py && ../../../para.sh finish "A"
cd ../20*/                           # Session 2: conflicting change
echo 'change B' >> test-file.py && ../../../para.sh finish "B"  # Conflict!
# Edit file to resolve conflicts, then:
./para.sh continue                 # Auto-stages & completes
```

### Module Structure

Each module in `lib/` has a specific responsibility:

- **para-config.sh**: Environment setup and configuration loading
- **para-utils.sh**: Common utilities, validation, and helper functions
- **para-git.sh**: All Git operations including worktree and finish management
- **para-session.sh**: Session lifecycle, state management, and detection
- **para-ide.sh**: IDE integration with extensible interface

## 🤝 Contributing

The modular architecture makes contributions easier:

1. **Bug fixes**: Usually isolated to a single module
2. **New features**: Can often be added by extending existing modules
3. **IDE support**: Add new implementations to `para-ide.sh`
4. **Git workflows**: Extend `para-git.sh` for new finish strategies

## 📋 Requirements

- **Git** with worktree support (Git 2.5+)
- **POSIX shell** (bash, zsh, fish, dash, ash, etc.)
- **Your preferred IDE** with CLI support (configure via `IDE_CMD`)

## 🐛 Troubleshooting

### Common Issues

**"not in a Git repository"**
- Run para from within a Git repository

**"IDE CLI not found"**
- Install your IDE's CLI or set `IDE_CMD` environment variable
- For Claude Code: ensure `claude` command is in PATH
- For Cursor: ensure `cursor` command is in PATH
- For VS Code: ensure `code` command is in PATH

**"session not found"**
- Use `para list` to see active sessions
- Ensure you're in the correct directory for auto-detection

### IDE-Specific Setup

**Claude Code:**
- Install Claude Code with CLI support
- Run `claude --help` to verify CLI is working

**Cursor:**
- Install Cursor CLI: https://cursor.sh/cli
- Run `cursor --help` to verify CLI is working

**VS Code:**
- Install VS Code with CLI: `code --install-extension` should work
- Or install via: https://code.visualstudio.com/docs/setup/mac#_launching-from-the-command-line