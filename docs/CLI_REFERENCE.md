# Para CLI Reference

Complete reference for all Para commands and options.

## Overview

Para is a parallel IDE workflow helper that enables working on multiple features simultaneously using Git worktrees. It's designed for AI-assisted development workflows, allowing multiple instances to work on different branches without conflicts.

## Command Structure

```
para [COMMAND] [OPTIONS]
```

When run without any command, Para opens the monitor view to manage active sessions.

## Core Commands

### `para start`

Creates new para sessions (interactive or AI-assisted).

**Usage:**
```bash
# Start new interactive session
para start
para start feature-xyz

# Start new session with AI agent (requires -p/--prompt or -f/--file)
para start -p "implement user authentication"
para start --prompt "implement authentication"
para start feature-xyz -p "implement feature"

# Use task file
para start -f tasks/auth.md
para start --file context.md
para start feature-xyz -f tasks.md

# Docker container sessions
para start --container -p "implement feature"
para start --container --allow-domains github.com,api.example.com -p "fetch data"
```

**Options:**
- `name` - Optional session name (auto-generated if not provided)
- `-p, --prompt <TEXT>` - Prompt for AI-assisted session
- `-f, --file <PATH>` - Read prompt/context from specified file
- `-d, --dangerously-skip-permissions` - Skip IDE permission warnings (dangerous)
- `-c, --container` - Run session in Docker container
- `--allow-domains <DOMAINS>` - Enable network isolation with allowed domains (comma-separated)
- `--docker-args <ARGS>` - Additional Docker arguments to pass through
- `--setup-script <PATH>` - Path to setup script to run after session creation
- `--docker-image <IMAGE>` - Custom Docker image to use (e.g., 'ubuntu:22.04')
- `--no-forward-keys` - Disable automatic API key forwarding to containers
- `-s, --sandbox` - Enable sandboxing (overrides config)
- `--no-sandbox` - Disable sandboxing (overrides config)
- `--sandbox-profile <PROFILE>` - Sandbox profile: permissive (default) or restrictive
- `--sandbox-no-network` - Enable network-isolated sandboxing
- `--allowed-domains <DOMAINS>` - Additional domains for network proxy (comma-separated)

**Validation Rules:**
- Session names must be 50 characters or less
- Session names can only contain alphanumeric characters, hyphens, and underscores
- Session names cannot be empty
- Cannot create a session with a name that already exists

**Examples:**
```bash
# Interactive session
para start my-feature

# AI-assisted session from prompt
para start -p "Add JWT authentication to the API"

# AI-assisted session from file
para start -f ./tasks/implement-auth.md

# Named AI-assisted session
para start auth-feature -p "Add user authentication"

# Container session with network isolation
para start --container --allow-domains npmjs.org,github.com -p "Install dependencies"
```

### `para finish`

Complete session and create feature branch for review.

**Usage:**
```bash
para finish "commit message"
para finish "implement user auth" --branch custom-branch-name
para finish "fix login bug" my-session
```

**Arguments:**
- `message` - Commit message (required, cannot be empty)
- `session` - Session ID (optional, auto-detects if not provided)

**Options:**
- `-b, --branch <NAME>` - Custom branch name after finishing

**Branch Validation Rules:**
- Branch names cannot be empty
- Branch names cannot start or end with hyphen
- Branch names cannot contain `..` or `//`
- Branch names cannot be longer than 250 characters
- Branch names cannot contain control characters, spaces, or special Git characters (~, ^, :, ?, *, [, \, @, {)
- Branch names cannot start with `refs/`
- Branch names cannot end with `/`

**Examples:**
```bash
# Finish current session
para finish "Add user authentication system"

# Finish with custom branch name
para finish "Fix login validation" --branch hotfix-login-validation

# Finish specific session
para finish "Update API endpoints" auth-session
```

### `para resume`

Resume session in IDE with optional additional context.

**Usage:**
```bash
# Resume session from current directory (auto-detect)
para resume

# Resume specific session
para resume my-feature

# Resume with additional instructions
para resume my-feature --prompt "add error handling"

# Resume with instructions from file
para resume my-feature --file new-requirements.txt
```

**Arguments:**
- `session` - Session ID (optional, auto-detects from current directory if not provided)

**Options:**
- `--prompt <TEXT>` - Additional prompt or instructions for the resumed session
- `--file <PATH>` - Read additional instructions from specified file
- `--dangerously-skip-permissions` - Skip IDE permission warnings (dangerous)
- Sandbox options (same as `para start`)

**Validation:**
- Cannot specify both `--prompt` and `--file`
- Session identifier cannot be empty

**Examples:**
```bash
# Resume current session
para resume

# Resume with additional context
para resume auth-feature --prompt "Add password reset functionality"

# Resume with requirements from file
para resume api-feature --file additional-requirements.md
```

### `para list`

List active sessions.

**Usage:**
```bash
para list
para ls  # alias
```

**Options:**
- `-v, --verbose` - Show verbose session information
- `-a, --archived` - Show archived sessions
- `-q, --quiet` - Quiet output for completion

**Examples:**
```bash
# List all active sessions
para list

# Show detailed information
para list --verbose

# Include archived sessions
para list --archived
```

### `para cancel`

Cancel session (moves to archive).

**Usage:**
```bash
para cancel
para cancel my-session
```

**Arguments:**
- `session` - Session ID (optional, auto-detects if not provided)

**Options:**
- `-f, --force` - Force cancellation even with uncommitted changes (destructive)

**Examples:**
```bash
# Cancel current session
para cancel

# Cancel specific session
para cancel my-feature

# Force cancel with uncommitted changes
para cancel my-feature --force
```

### `para clean`

Remove all active sessions.

**Usage:**
```bash
para clean
```

**Options:**
- `-f, --force` - Skip confirmation prompts
- `--dry-run` - Only show what would be cleaned (dry run)
- `--containers` - Clean orphaned Docker containers

**Examples:**
```bash
# Clean all sessions with confirmation
para clean

# Clean without confirmation
para clean --force

# Show what would be cleaned
para clean --dry-run

# Clean containers too
para clean --containers
```

### `para recover`

Recover cancelled session from archive.

**Usage:**
```bash
para recover
para recover my-session
```

**Arguments:**
- `session` - Session ID to recover from archive (optional, shows list if not provided)

**Recovery Process:**
1. Validates that the session can be recovered
2. Checks for conflicts with existing sessions/branches
3. Prompts for confirmation if warnings exist
4. Recreates the session worktree and branch
5. Restores session state

**Examples:**
```bash
# Show recoverable sessions and select interactively
para recover

# Recover specific session
para recover my-feature
```

### `para monitor`

Monitor and manage active sessions in real-time (interactive TUI with mouse support).

**Usage:**
```bash
para monitor
para  # default command when no subcommand provided
```

**Features:**
- Real-time session monitoring
- Interactive TUI with mouse support
- Session status updates
- Container management
- Process monitoring

### `para status`

Update session status (for agents to communicate progress) or show status information.

**Usage:**
```bash
# Update current session status
para status "Working on authentication" --tests passed --todos 3/5

# Update specific session
para status "Debugging login" --tests failed --session my-feature --blocked

# Show status of all sessions
para status show

# Show status of specific session
para status show my-feature

# Show status as JSON
para status show --json

# Show summary of all sessions
para status summary

# Clean up stale status files
para status cleanup
```

**Update Options:**
- `task` - Current task description (required for updates)
- `--tests <STATUS>` - Test status: passed, failed, or unknown (required for updates)
- `--todos <PROGRESS>` - Todo progress in format 'completed/total' (e.g., '3/7')
- `--blocked` - Mark session as blocked
- `--session <NAME>` - Session name (auto-detected if not provided)

**Show/Summary Options:**
- `show [session]` - Show status of one or all sessions
- `summary` - Generate a summary of all status files
- `cleanup` - Clean up stale status files
- `--json` - Output as JSON
- `--dry-run` - Show what would be cleaned without removing

**Examples:**
```bash
# Update status from within session directory
para status "Implementing JWT tokens" --tests passed --todos 2/5

# Update status for specific session
para status "Debugging Redis connection" --tests failed --session auth-feature --blocked

# Show all session statuses
para status show

# Show specific session status as JSON
para status show auth-feature --json

# Show summary of all sessions
para status summary

# Clean stale status files
para status cleanup --dry-run
```

## Configuration Commands

### `para config`

Setup configuration.

**Usage:**
```bash
para config [SUBCOMMAND]
```

**Subcommands:**
- `setup` - Interactive configuration wizard
- `auto` - Auto-detect and configure IDE
- `show` - Show current configuration
- `edit` - Edit configuration file
- `reset` - Reset configuration to defaults
- `set <path> <value>` - Set configuration value using JSON path
- `project [SUBCOMMAND]` - Manage project-level configuration

**Project Subcommands:**
- `init` - Initialize project configuration
- `show` - Show project configuration
- `edit` - Edit project configuration
- `set <path> <value>` - Set project configuration value

**Examples:**
```bash
# Interactive setup
para config setup

# Auto-detect IDE
para config auto

# Show current config
para config show

# Set IDE preference
para config set ide.name cursor

# Initialize project config
para config project init
```

### `para auth`

Manage Docker container authentication.

**Usage:**
```bash
para auth [SUBCOMMAND]
```

**Subcommands:**
- `setup` - Set up container authentication interactively
- `cleanup` - Remove authentication artifacts
- `status` - Check authentication status
- `reauth` - Re-authenticate (cleanup and setup in one command)

**Options:**
- `--force` - Force re-authentication even if credentials exist (for setup)
- `--dry-run` - Show what would be removed without actually removing (for cleanup)
- `--verbose` - Show detailed authentication information (for status)

**Examples:**
```bash
# Set up authentication
para auth setup

# Check authentication status
para auth status --verbose

# Clean up authentication
para auth cleanup

# Re-authenticate
para auth reauth
```

### `para mcp`

Setup Model Context Protocol (MCP) integration.

**Usage:**
```bash
para mcp init
```

**What it does:**
- Creates `.mcp.json` with Para MCP server configuration
- Adds `.mcp.json` to `.gitignore`
- Enables Para tools in IDEs that support MCP

**Examples:**
```bash
# Initialize MCP integration
para mcp init
```

## Utility Commands

### `para completion`

Generate shell completion script.

**Usage:**
```bash
para completion <SHELL>
```

**Supported shells:**
- bash
- zsh
- fish

**Examples:**
```bash
# Generate bash completion
para completion bash

# Generate zsh completion
para completion zsh

# Install completion (example for bash)
para completion bash > /usr/local/etc/bash_completion.d/para
```

### `para init`

Initialize shell completions automatically.

**Usage:**
```bash
para init
```

This command sets up shell completions for the current shell environment.

## Docker Integration

Para supports running sessions in Docker containers for isolation and reproducibility.

### Container Options

All session commands (`start`, `resume`) support these Docker options:

- `--container` - Run session in Docker container
- `--allow-domains <DOMAINS>` - Enable network isolation with allowed domains
- `--docker-args <ARGS>` - Additional Docker arguments
- `--docker-image <IMAGE>` - Custom Docker image
- `--no-forward-keys` - Disable API key forwarding
- `--setup-script <PATH>` - Setup script to run after creation

### Network Isolation

When using `--allow-domains`, Para creates network-isolated containers that only allow access to specified domains plus default domains (GitHub, npmjs.org, etc.).

**Examples:**
```bash
# Container with full network access
para start --container -p "implement feature"

# Network-isolated container
para start --container --allow-domains api.example.com -p "fetch data"

# Custom image with setup script
para start --container --docker-image ubuntu:20.04 --setup-script ./setup.sh
```

## Sandboxing

Para supports sandboxing for security when running AI agents.

### Sandbox Options

- `--sandbox` - Enable sandboxing (file write restrictions)
- `--no-sandbox` - Disable sandboxing
- `--sandbox-profile <PROFILE>` - Sandbox profile (permissive/restrictive)
- `--sandbox-no-network` - Enable network-isolated sandboxing
- `--allowed-domains <DOMAINS>` - Additional domains for network proxy

### Sandbox Profiles

- **permissive** (default) - Restricts file writes to session directory, full network access
- **restrictive** - Network limited to GitHub API only via proxy

**Examples:**
```bash
# Basic sandboxing
para start --sandbox -p "implement feature"

# Network-isolated sandboxing
para start --sandbox-no-network -p "safe implementation"

# Custom network domains
para start --sandbox-no-network --allowed-domains npmjs.org,pypi.org -p "install packages"
```

## Global Options

These options are available for most commands:

- `--help` - Show help information
- `--version` - Show version information

## Session Names and Branch Names

### Session Name Rules

- Maximum 50 characters
- Only alphanumeric characters, hyphens, and underscores
- Cannot be empty
- Cannot contain spaces or special characters

### Branch Name Rules

- Maximum 250 characters
- Cannot be empty
- Cannot start or end with hyphen
- Cannot contain: `..`, `//`, control characters, spaces
- Cannot contain special Git characters: `~`, `^`, `:`, `?`, `*`, `[`, `\`, `@`, `{`
- Cannot start with `refs/`
- Cannot end with `/`

## Error Handling

Para provides detailed error messages with context for common issues:

- **Session not found** - Suggests available sessions
- **Invalid session names** - Explains validation rules
- **Git conflicts** - Provides resolution steps
- **Container issues** - Docker-specific troubleshooting
- **Sandbox violations** - Security-related errors

## Exit Codes

- `0` - Success
- `1` - General error
- `2` - Invalid arguments
- `3` - Session not found
- `4` - Git operation failed
- `5` - Configuration error

## Environment Variables

- `PARA_CONFIG_PATH` - Override config file location
- `PARA_NON_INTERACTIVE` - Disable interactive prompts
- `CI` - Automatically detected for CI environments

## Examples

### Basic Workflow

```bash
# Start new feature
para start feature-auth -p "Implement user authentication"

# Check status
para status show

# Finish when done
para finish "Add JWT authentication with password reset"
```

### Container Workflow

```bash
# Start in container
para start --container --allow-domains github.com,npmjs.org -p "Set up React app"

# Resume with additional context
para resume --prompt "Add TypeScript support"

# Finish
para finish "Set up React app with TypeScript"
```

### Recovery Workflow

```bash
# Cancel session
para cancel my-feature

# Later, recover it
para recover my-feature

# Resume work
para resume my-feature --prompt "Continue where I left off"
```

### Multi-Session Management

```bash
# List all sessions
para list --verbose

# Monitor in real-time
para monitor

# Check status summary
para status summary

# Clean up old sessions
para clean --dry-run
para clean --force
```

## Tips

1. **Use descriptive session names** - Makes it easier to manage multiple sessions
2. **Leverage file input** - Use `--file` for complex prompts and requirements
3. **Monitor sessions** - Use `para monitor` for real-time session management
4. **Network isolation** - Use `--allow-domains` for secure AI agent execution
5. **Recovery safety** - Sessions are archived, not deleted, so you can always recover
6. **Status tracking** - Use `para status` commands for team coordination
7. **Container isolation** - Use `--container` for reproducible development environments

## See Also

- [Configuration Guide](DETAILED_CONFIGURATION.md)
- [Workflow Guide](WORKFLOW.md)
- [MCP Integration](MCP_INTEGRATION.md)
- [Troubleshooting](TROUBLESHOOTING.md)
- [Docker Integration](docker-integration-design.md)