# Detailed Configuration

## Configuration Hierarchy

Para supports hierarchical configuration with the following precedence (highest to lowest):

1. **Command-line arguments** - Override everything for individual commands
2. **Project configuration** - `.para/config.json` in your repository
3. **User configuration** - Your personal Para config file
4. **System defaults** - Built-in Para defaults

**How it works**: Para loads BOTH your user config and project config (if present), then intelligently merges them together. Project settings override user settings for specific fields, while arrays are combined. This allows teams to share project-specific settings while maintaining personal preferences.

## Configuration File Locations

### User Configuration

Para stores your personal configuration in platform-specific directories:

- **macOS**: `~/Library/Application Support/para/config.json`
- **Linux**: `~/.config/para/config.json`
- **Windows**: `%APPDATA%\para\config.json`

The exact location is determined by the `directories` crate using `ProjectDirs::from("", "", "para")`.

### Project Configuration

Project configuration is stored in the repository itself:

- **Location**: `.para/config.json` in your repository root
- **Purpose**: Share configuration settings across your team
- **Version control**: **Should be committed** to your repository

Para automatically searches for project configuration by walking up the directory tree from your current location until it finds a `.para/config.json` file.

## Configuration Management Commands

### User Configuration

```bash
# Show current merged configuration (user + project)
para config show

# Edit user configuration file
para config edit

# Set specific configuration values
para config set ide.name "cursor"
para config set git.branch_prefix "feature"
para config set session.auto_cleanup_days 14

# Reset to default configuration
para config reset

# Run interactive configuration wizard
para config setup
```

### Project Configuration

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

## Project-Level Configuration

Share configuration settings across your team by creating a `.para/config.json` file in your repository. Unlike user config, this file **should be committed** to your repository so all team members use the same settings.

### Project Configuration Options

Project configuration is **partial** - you only need to specify what you want to override or add. All other settings from the user config are preserved.

**Sandbox Settings:**
- `sandbox.enabled` - Enable/disable sandboxing for all team members
- `sandbox.profile` - Default sandbox profile (`standard`, `permissive-open`, `standard-proxied`)
- `sandbox.allowed_domains` - Array of additional allowed domains

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

## Complete Configuration Structure

Para's configuration file contains the following sections:

### IDE Configuration

```json
{
  "ide": {
    "name": "claude",
    "command": "claude",
    "user_data_dir": null,
    "wrapper": {
      "enabled": true,
      "name": "code",
      "command": "code"
    }
  }
}
```

**Fields:**
- `name`: IDE identifier (claude, cursor, code, etc.)
- `command`: Command to launch the IDE
- `user_data_dir`: Custom user data directory name (optional)
- `wrapper.enabled`: Whether to use wrapper mode
- `wrapper.name`: Wrapper IDE name
- `wrapper.command`: Wrapper IDE command

### Directory Configuration

```json
{
  "directories": {
    "subtrees_dir": ".para/worktrees",
    "state_dir": ".para/state"
  }
}
```

**Fields:**
- `subtrees_dir`: Directory for git worktrees (relative to repository root)
- `state_dir`: Directory for Para state files (relative to repository root)

### Git Configuration

```json
{
  "git": {
    "branch_prefix": "para",
    "auto_stage": true,
    "auto_commit": true
  }
}
```

**Fields:**
- `branch_prefix`: Prefix for branch names
- `auto_stage`: Automatically stage all changes when finishing
- `auto_commit`: Automatically commit changes when finishing

### Session Configuration

```json
{
  "session": {
    "default_name_format": "%Y%m%d-%H%M%S",
    "preserve_on_finish": false,
    "auto_cleanup_days": 30
  }
}
```

**Fields:**
- `default_name_format`: Default session name format (strftime format)
- `preserve_on_finish`: Keep session after finishing
- `auto_cleanup_days`: Auto-cleanup sessions after N days (optional)

### Docker Configuration

```json
{
  "docker": {
    "setup_script": "/path/to/setup.sh",
    "default_image": "ubuntu:22.04",
    "forward_env_keys": [
      "ANTHROPIC_API_KEY",
      "OPENAI_API_KEY",
      "GITHUB_TOKEN"
    ]
  }
}
```

**Fields:**
- `setup_script`: Path to Docker setup script (optional)
- `default_image`: Default Docker image to use (optional)
- `forward_env_keys`: Environment variables to forward to containers (optional)

### Sandbox Configuration

```json
{
  "sandbox": {
    "enabled": false,
    "profile": "permissive-open",
    "allowed_domains": [
      "github.com",
      "npmjs.org"
    ]
  }
}
```

**Fields:**
- `enabled`: Enable sandboxing
- `profile`: Sandbox profile (`standard`, `permissive-open`, `standard-proxied`)
- `allowed_domains`: Additional allowed domains for network access

## IDE Configuration

Para supports multiple IDEs with configurable wrapper mode:

### Claude Code (AI Development - Wrapper Mode Required)

Claude Code must be launched **inside** VS Code or Cursor to work with Para. This gives you both a full IDE interface and Claude Code running automatically in the integrated terminal:

```bash
# Configure via Para config
para config set ide.name "claude"
para config set ide.command "claude"
para config set ide.wrapper.enabled true
para config set ide.wrapper.name "code"      # Use VS Code as wrapper
para config set ide.wrapper.command "code"

# Or use Cursor as wrapper
para config set ide.wrapper.name "cursor"    # Use Cursor as wrapper
para config set ide.wrapper.command "cursor"
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

### VS Code

```bash
para config set ide.name "code"
para config set ide.command "code"
para config set ide.user_data_dir ".vscode-userdata"
```

### Cursor

```bash
para config set ide.name "cursor"
para config set ide.command "cursor"
para config set ide.user_data_dir ".cursor-userdata"
```

### Custom IDE

```bash
para config set ide.name "my-editor"
para config set ide.command "my-editor-command"
para config set ide.user_data_dir ".my-editor-userdata"
```

## Environment Variables

Para supports a limited set of environment variables for specific use cases:

### Para Behavior

```bash
# Non-interactive mode (for CI/automation)
export PARA_NON_INTERACTIVE=1

# Configuration file override for testing
export PARA_CONFIG_PATH="/path/to/custom/config.json"

# Completion script mode
export PARA_COMPLETION_SCRIPT=1
export PARA_COMPLETION_HELP=1
```

### System Environment

```bash
# Standard environment variables Para respects
export SHELL="/bin/zsh"           # Shell detection
export EDITOR="nano"              # Default editor for config edit
export CI=1                       # CI environment detection
```

**Note**: The legacy `IDE_NAME`, `IDE_CMD`, and related environment variables are **no longer supported**. Use the configuration file instead with `para config set` commands.

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
para config set ide.user_data_dir ".my-custom-data"
para start                  # Uses custom directory name
```

**Disable isolation (use main IDE instance):**
```bash
para config set ide.user_data_dir null
para start                  # Uses your main IDE instance
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