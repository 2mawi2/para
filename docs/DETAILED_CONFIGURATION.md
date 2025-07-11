# Detailed Configuration

## Configuration Hierarchy

Para supports hierarchical configuration with the following precedence (highest to lowest):

1. **Command-line arguments** - Override everything for individual commands
2. **Project configuration** - `.para/config.json` in your repository
3. **User configuration** - Your personal Para config
4. **System defaults** - Built-in Para defaults

**How it works**: Para loads BOTH your user config and project config (if present), then intelligently merges them together. Project settings override user settings for specific fields, while arrays are combined. This allows teams to share project-specific settings while maintaining personal preferences.

## Project-Level Configuration

Share configuration settings across your team by creating a `.para/config.json` file in your repository. Unlike user config, this file **should be committed** to your repository so all team members use the same settings.

### Quick Setup
```bash
# Initialize project configuration with defaults
para config project init

# Show current project configuration
para config project show

# Edit project configuration
para config project edit

# Set specific project values
para config project set sandbox.enabled true
para config project set sandbox.allowed_domains "api.company.com,internal.service.com"
para config project set ide.preferred "cursor"
```

### Project Configuration Options

Project configuration is **partial** - you only need to specify what you want to override or add. All other settings from the user config are preserved.

**Sandbox Settings:**
- `sandbox.enabled` - Enable/disable sandboxing for all team members
- `sandbox.profile` - Default sandbox profile (standard, permissive, restrictive)
- `sandbox.allowed_domains` - Comma-separated list of additional allowed domains

**IDE Preferences:**
- `ide.preferred` - Preferred IDE for this project (overrides user preference)

### Example Project Configuration

`.para/config.json`:
```json
{
  "sandbox": {
    "enabled": true,
    "profile": "standard", 
    "allowed_domains": [
      "api.internal.com",
      "docs.company.com",
      "registry.npmjs.org"
    ]
  },
  "ide": {
    "preferred": "cursor"
  }
}
```

### Configuration Merging

When you run Para commands, **both** user and project configurations are loaded and merged:

1. **User config is loaded first** from your personal config file
2. **Project config is loaded** if `.para/config.json` exists in the repository
3. **Smart merging happens**:
   - **Sandbox enabled/profile**: Project overrides user settings
   - **Allowed domains**: Project domains are **added** to user domains (merged and deduplicated)
   - **IDE preference**: Project preference overrides user preference
   - **Other settings**: User config values are preserved if not specified in project config

**Example merge:**
```bash
# User config has:
#   sandbox.enabled: false
#   sandbox.allowed_domains: ["github.com", "gitlab.com"]
#   ide.name: "claude"
#
# Project config has:
#   sandbox.enabled: true
#   sandbox.allowed_domains: ["api.internal.com", "github.com"]
#   ide.preferred: "cursor"
#
# Final merged config:
#   sandbox.enabled: true (from project)
#   sandbox.allowed_domains: ["api.internal.com", "github.com", "gitlab.com"] (merged)
#   ide.name: "cursor" (from project.ide.preferred)
```

### Team Workflow

1. **Project lead** sets up configuration:
   ```bash
   para config project init
   para config project set sandbox.enabled true
   para config project set sandbox.allowed_domains "api.company.com"
   git add .para/config.json
   git commit -m "Add Para project configuration"
   ```

2. **Team members** automatically inherit settings:
   ```bash
   git pull
   para start "implement feature"  # Uses project config automatically
   ```

3. **Individual overrides** still work:
   ```bash
   para start --no-sandbox "debug without sandbox"  # CLI override
   ```

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