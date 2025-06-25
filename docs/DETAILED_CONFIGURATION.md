# Detailed Configuration

## IDE Configuration

Para supports multiple IDEs with easy configuration:

### Claude Code (AI Development - Wrapper Mode Only)
```bash
export IDE_NAME="claude"
export IDE_CMD="claude"
# Note: Claude Code doesn't support --user-data-dir isolation
```

#### IDE Wrapper for Claude Code (Required)

Claude Code must be launched **inside** VS Code or Cursor to work with Para. This gives you both a full IDE interface and Claude Code running automatically in the integrated terminal:

```bash
# Configure Claude Code with VS Code wrapper
export IDE_NAME="claude"
export IDE_CMD="claude"
export IDE_WRAPPER_ENABLED="true"
export IDE_WRAPPER_NAME="code"      # Use VS Code as wrapper
export IDE_WRAPPER_CMD="code"

# Or use Cursor as wrapper
export IDE_WRAPPER_NAME="cursor"    # Use Cursor as wrapper
export IDE_WRAPPER_CMD="cursor"
```

**How it works:**
1. `para start` opens your chosen IDE (VS Code/Cursor) with the worktree
2. The IDE automatically runs `claude` in its integrated terminal
3. You get the best of both worlds: full IDE features + Claude Code

**First-time setup:**
- VS Code will prompt *"Allow automatic tasks?"* the first time
- Select **Allow** to enable the automatic Claude Code startup
- This only needs to be done once per VS Code installation

**Benefits:**
- **Zero-click workflow** - Claude Code starts automatically
- **Full IDE interface** - file explorer, git integration, extensions
- **Integrated terminal** - Claude Code runs in the IDE's terminal
- **Better UX** - no separate terminal windows to manage

### VS Code (Default)
```bash
export IDE_NAME="code"    # Default
export IDE_CMD="code"     # Default
export IDE_USER_DATA_DIR=".vscode-userdata"  # Default
```

### Cursor
```bash
export IDE_NAME="cursor"
export IDE_CMD="cursor"
export IDE_USER_DATA_DIR=".cursor-userdata"
```

### Custom IDE
```bash
export IDE_NAME="my-editor"
export IDE_CMD="my-editor-command"
export IDE_USER_DATA_DIR=".my-editor-userdata"
```

**Make configuration persistent:** Add these exports to your shell configuration file (`~/.bashrc`, `~/.zshrc`, `~/.config/fish/config.fish`, etc.)

## Environment Variables

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

## IDE Isolation Feature

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

## IDE-Specific Setup

**Claude Code:**
- Install Claude Code with CLI support
- Run `claude --help` to verify CLI is working

**Cursor:**
- Install Cursor CLI: https://cursor.sh/cli
- Run `cursor --help` to verify CLI is working

**VS Code:**
- Install VS Code with CLI: `code --install-extension` should work
- Or install via: https://code.visualstudio.com/docs/setup/mac#_launching-from-the-command-line 