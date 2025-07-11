# Para - Parallel IDE Workflow Helper

Work on multiple features simultaneously using Git worktrees and your favorite IDE. Built with Rust for performance and reliability.

## Installation

```bash
# Homebrew (recommended)
brew install 2mawi2/tap/para

# Or build from source
git clone https://github.com/2mawi2/para.git
cd para
cargo build --release
sudo cp target/release/para /usr/local/bin/
```

## Quick Start

```bash
# Configure your IDE (one-time setup)
para config

# Optional: Enable shell completion for faster workflow
para init

# Create a new session (opens your IDE)
para start

# Work in the new IDE window...

# Create feature branch for review
para finish "Add new feature"
```

## Why Para?

**Problem:** You want to work on multiple features, experiments, or bug fixes at the same time, but switching Git branches disrupts your workflow and mixes up uncommitted changes.

**Solution:** Para creates isolated development environments (separate directories + Git branches) so you can:
- Have multiple IDE windows open, each working on different features
- Switch instantly between projects without losing context
- Let AI agents work in parallel without interfering with each other
- Keep your main branch always clean

**Perfect for:** AI-assisted development, feature prototyping, parallel experiments, or any workflow where you need multiple isolated workspaces.

## Core Commands

### Session Management
- `para start [name]` - Create new parallel session (opens configured IDE)
- `para finish "message"` - Auto-stage & finish session with commit message
- `para finish "message" --branch custom-name` - Finish with custom branch name
- `para list` - Show all active sessions
- `para cancel [session] [--force]` - Discard current or specified session (use --force to skip confirmation)
- `para clean` - Remove all sessions
- `para resume [session]` - Resume session in IDE
- `para resume [session] --prompt "text"` - Resume with additional instructions
- `para resume [session] --file context.md` - Resume with context from file
- `para recover [session]` - Recover cancelled session from backup

### Configuration
- `para config` - Interactive configuration wizard
- `para config auto` - Auto-detect IDE and create config
- `para config show` - Display current settings
- `para config edit` - Edit configuration file
- `para config reset` - Reset configuration to defaults

#### Project-Level Configuration
- `para config project init` - Create `.para/config.json` for team-shared settings
- `para config project show` - Display project configuration
- `para config project edit` - Edit project configuration file
- `para config project set <path> <value>` - Set project configuration values

## AI Integration

### Claude Code MCP Integration

Para integrates seamlessly with Claude Code through MCP (Model Context Protocol):

```bash
# One-time setup (navigate to any repo where you want Para tools)
para mcp init

# This creates .mcp.json with the correct paths for your system
# (Note: .mcp.json contains user-specific paths, so add it to .gitignore)

# Now Claude Code has Para tools available:
# - para_start, para_finish, para_resume, para_list, etc.
# - Use Para tools directly from Claude Code interface
```

### AI-Powered Development
```bash
para start -p "prompt"                   # Create session with AI prompt
para start --prompt "prompt"             # Create session with AI prompt (long form)
para start name -p "prompt"              # Named session with AI prompt
para start --file prompt.txt             # Create session with prompt from file
para start -f ./auth.prompt               # Create session with prompt from file (short form)

# Skip permission warnings in trusted environments (CI, scripts)
para start --dangerously-skip-permissions -p "prompt"
para start --dangerously-skip-permissions name
```

## Custom Branch Names

Para supports custom branch names when finishing sessions:

```bash
# Default behavior - uses session name with timestamp
para finish "Implement user authentication"
# Creates branch: para/session-name-20240531-184623

# Custom branch name
para finish "Implement auth" --branch feature-authentication
# Creates branch: feature-authentication

# Branch conflict resolution
para finish "Fix bug" --branch existing-feature
# If 'existing-feature' exists, creates: existing-feature-1
```

### Branch Validation
- Branch names must be valid Git branch names
- Cannot contain spaces or special characters (~ ^ : ? * [ \ @)
- Cannot start with `-` or `.`
- Cannot end with `/`
- Cannot contain sequences like `..`, `@{`, `//`, or `/.`

The `start` command with the `-p/--prompt` flag creates new sessions and immediately opens Claude Code with your prompt, perfect for AI-assisted development.

**MCP Integration:** After running `para mcp init --claude-code` in your repo, Claude Code gains native Para tools for session management.

**Note:** AI-assisted sessions (start with prompts) only work with Claude Code. Use `para config` to switch IDEs if needed.

## How It Works

1. **Configure**: `para config` detects and configures your IDE (Cursor, Claude Code, VS Code, etc.)
2. **Create**: `para start` creates a timestamped Git branch and separate directory
3. **Work**: Each session gets its own IDE window and isolated workspace
4. **Finish**: `para finish` automatically stages changes and creates feature branch for review
5. **Clean**: Sessions clean up automatically

**File Structure:**
```
your-repo/
├── .git/                    # Your main repo
└── .para/                   # Para directory (auto-managed)
    ├── .gitignore           # Prevents tracking Para files  
    ├── state/               # Session state files
    └── worktrees/           # Para sessions live here
        ├── feature-auth-*/  # Session 1: authentication work
        ├── feature-ui-*/    # Session 2: UI updates  
        └── bugfix-login-*/  # Session 3: bug fix
```

## IDE Setup

Run `para config` to set up your IDE - it will auto-detect and configure:

- **Claude Code** (recommended for AI development)
- **VS Code** (default)
- **Cursor** 
- **Any IDE with CLI support**

```bash
para config              # Interactive setup wizard
para config auto         # Auto-detect IDE  
para config show         # Show current settings
para config edit         # Edit config file
```

### Configuration File Locations

Para stores configuration files in platform-specific locations:

**Rust Implementation:**
- **macOS:** `~/Library/Application Support/para/config.json`
- **Linux:** `~/.config/para/config.json`
- **Windows:** `%APPDATA%\para\config.json`

Para uses JSON format and is automatically created on first run.

## Sandbox Security (macOS)

Para supports sandboxing for Claude CLI sessions on macOS, providing protection against prompt injection attacks by limiting file write access.

### Configuring Sandbox

**Enable by default (recommended for AI development):**
```bash
para config set sandbox.enabled true
```

**Team-wide configuration (recommended for shared projects):**
```bash
# Enable sandbox for all team members
para config project set sandbox.enabled true

# Add project-specific allowed domains
para config project set sandbox.allowed_domains "api.company.com,docs.internal.com"
```

**Check current settings:**
```bash
para config show | grep -A 2 sandbox
```

### Sandbox Profile

Para uses a single standard profile that protects against malicious file writes while allowing Claude to function normally:

**Allowed write locations:**
- Project directory (current repo)
- Temporary files (/tmp, /var/folders)
- Claude configuration (~/.claude)
- Git configuration (~/.gitconfig)
- Cache directories (~/.cache)

**Blocked write locations:**
- Home directory (except allowed paths)
- System directories
- SSH keys (~/.ssh)
- Shell configurations (~/.bashrc, ~/.zshrc)
- Other sensitive locations

**Always allowed:**
- Full read access to all files
- Network access for Claude API
- Process execution

### Per-Session Control

Override default settings for specific sessions:
```bash
# Force sandbox on
para start my-session --sandbox
para start my-task -p "implement feature" --sandbox

# Force sandbox off (use with caution)
para start my-session --no-sandbox

# Sessions remember their sandbox settings when resumed
para resume my-session  # Uses original sandbox settings
```

## Example Workflows

### Regular Parallel Development
```bash
# Start multiple parallel sessions
para start feature-auth     # Session 1: authentication
para start feature-ui       # Session 2: UI updates  
para start bugfix-login     # Session 3: bug fix

# Each opens in a separate IDE window
# Work in parallel without interference

# Finish sessions when ready (from any directory)
para finish "Add OAuth login"      # Finishes current session
para finish "Update dashboard UI"  # Finishes current session  
para finish "Fix login redirect"   # Finishes current session

# All features now ready for review as feature branches
```

### AI-Powered Development with MCP
```bash
# One-time setup in your repo
para mcp init --claude-code

# This creates .mcp.json (add to .gitignore - contains local paths)
echo ".mcp.json" >> .gitignore

# Claude Code now has native Para tools:
# - para_start: Create new sessions  
# - para_finish: Complete sessions with commits
# - para_list: View active sessions
# - para_resume: Resume existing sessions
```

### Traditional AI Development
```bash
# Create AI session with prompt
para start -p "Implement user authentication with best security practices"

# Or use a prompt file for complex prompts
para start --file auth-requirements.prompt

# Claude Code opens with your prompt for AI-assisted development

# Finish when ready
para finish "Implement OAuth authentication"
```


## Resuming Sessions with Context

You can resume existing sessions with additional context or updated instructions:

```bash
# Resume normally (opens IDE)
para resume my-session

# Resume with additional instructions
para resume my-session --prompt "Add unit tests for the auth module"

# Resume with context from a file
para resume my-session --file updated-requirements.md

# Resume from current directory (auto-detects session)
cd .para/worktrees/my-session
para resume --prompt "Continue where we left off"
```

The additional context is saved to `.para/sessions/{session}/resume_context.md` for reference.

## Session Recovery

Para automatically backs up the last 3 cancelled sessions for recovery:

```bash
# Cancel a session (automatically backed up)
para cancel my-session
# 💡 Session backed up for recovery. Use 'para recover my-session' to restore.

# View available backups
para recover
# Shows list of recoverable sessions

# Recover a cancelled session
para recover my-session
# ✅ recovered cancelled session my-session
# ↳ branch: para/my-session-20240531-184623
# ↳ worktree: .para/worktrees/my-session-20240531-184623
# ↳ resume: para resume my-session
```

**Note:** Only the last 3 cancelled sessions are kept as backups. The oldest backup is automatically removed when a new session is cancelled.

## Perfect For AI Development

Para is ideal when working with AI assistants:

- **Isolated workspaces**: Each session works independently without conflicts
- **Safe iteration**: Main branch stays clean while you experiment  
- **Easy comparison**: See results side-by-side in different IDE windows
- **Focused development**: Each session maintains its own context and state

## Documentation

- **[MCP Integration](docs/MCP_INTEGRATION.md)** - Complete guide to Claude Code MCP integration
- **[MCP Orchestration](docs/MCP_ORCHESTRATION.md)** - AI orchestration patterns and MCP tool documentation
- **[Sample Instructions](docs/SAMPLE_PARA_INSTRUCTIONS.md)** - Example CLAUDE.md for CLI-based workflows
- **[Detailed Configuration](docs/DETAILED_CONFIGURATION.md)** - Advanced IDE setup and environment variables
- **[Development Guide](docs/DEVELOPMENT.md)** - Contributing, architecture, testing
- **[Troubleshooting](docs/TROUBLESHOOTING.md)** - Common issues and solutions
- **[Workflow Guide](docs/WORKFLOW.md)** - Visual diagrams of Para workflows and state transitions
- **[IDE Behavior](docs/IDE_BEHAVIOR.md)** - IDE window management during integration and conflicts

## Requirements

- Git 2.5+ (for worktree support)
- Your preferred IDE with CLI support

## Environment Variables

Para supports several environment variables for configuration:

- **`IDE_NAME`** - IDE to use (`cursor`, `claude`, `code`, or custom)
- **`IDE_CMD`** - Command to launch the IDE
- **`IDE_USER_DATA_DIR`** - User data directory for IDE isolation
- **`BASE_BRANCH`** - Base branch for sessions (default: `main`)
- **`PARA_NON_INTERACTIVE`** - Skip interactive prompts (useful for CI/scripts)

### CI/Automation Usage

For automated environments, set `PARA_NON_INTERACTIVE=true` to skip welcome prompts:

```bash
export PARA_NON_INTERACTIVE=true
para start my-session
```

Para also auto-detects CI environments by checking for `CI` or `GITHUB_ACTIONS` environment variables.

## Shell Completion

Para provides intelligent tab completion for all commands, options, and dynamic data. The completion system is context-aware and helps with sessions, branches, files, and more.

### Quick Setup

**Automatic (Recommended):**
```bash
para init
# Auto-detects your shell and installs completions
# Restart your shell or follow the on-screen instructions
```

**Manual Setup:**

**Bash:**
```bash
mkdir -p ~/.local/share/bash-completion/completions
PARA_COMPLETION_SCRIPT=1 para completion bash > ~/.local/share/bash-completion/completions/para
# Restart shell or run: source ~/.local/share/bash-completion/completions/para
```

**Zsh:**
```bash
mkdir -p ~/.local/share/zsh/site-functions
PARA_COMPLETION_SCRIPT=1 para completion zsh > ~/.local/share/zsh/site-functions/_para
echo 'fpath=(~/.local/share/zsh/site-functions $fpath)' >> ~/.zshrc
echo 'autoload -U compinit && compinit' >> ~/.zshrc
# Restart shell
```

**Fish:**
```bash
mkdir -p ~/.config/fish/completions  
PARA_COMPLETION_SCRIPT=1 para completion fish > ~/.config/fish/completions/para.fish
# Restart shell or run: fish_update_completions
```

### Smart Completions

The completion system provides intelligent suggestions:

**📁 Session Management:**
- `para resume <TAB>` → Shows active sessions
- `para cancel <TAB>` → Shows active sessions  
- `para recover <TAB>` → Shows archived sessions

**🌿 Branch & Integration:**
- `para finish --branch <TAB>` → Shows git branches

**📄 File & Task Completion:**
- `para start --file <TAB>` → Prioritizes TASK_*.md files and .md files
- Smart file filtering for task-based workflows

**⚙️ Configuration:**
- `para config <TAB>` → Shows: `setup`, `auto`, `show`, `edit`, `reset`
- `para completion <TAB>` → Shows: `bash`, `zsh`, `fish`

**🎯 Flag Completion:**
- `para clean --<TAB>` → Shows: `--force`, `--dry-run`, `--backups`
- `para list --<TAB>` → Shows: `--verbose`, `--archived`, `--quiet`

### Homebrew Users

If you installed para via Homebrew, completions are automatically available! The formula includes completion caveats that guide you through the setup.

## Security Notes

The `--dangerously-skip-permissions` flag bypasses IDE permission warnings and should only be used in trusted environments like CI pipelines or automation scripts. It works with `start` and `resume` commands.

**⚠️ Use with caution** - this flag may allow IDEs to access system resources without permission prompts.

## Sandboxing Options

When using `--dangerously-skip-permissions` for autonomous agents, be aware of the security implications. Para provides multiple sandboxing levels - choose based on your project needs and environment.

### Available Security Levels

**1. Basic Sandboxing** (`--sandbox`, macOS only)
- **File writes**: Restricted to session worktree, temp directories, and specific config files
- **File reads**: Unrestricted (agents need to analyze codebases)
- **Network**: Full access

```bash
para start -p "implement feature" --sandbox
```

**2. Network-Isolated Sandboxing** (`--sandbox-no-network`, macOS only)
- **File writes**: Same restrictions as basic sandboxing
- **File reads**: Unrestricted
- **Network**: Limited to GitHub/Git operations and Anthropic API by default
- **Additional domains**: Use `--allow-domains "example.com,api.openai.com"`
- **True isolation**: Use `--allow-domains ""` to block all network access

```bash
para start -p "add tests" --sandbox-no-network
para start -p "fetch data" --sandbox-no-network --allow-domains "example.com"
```

**3. Docker Containerization** (`--container`, all platforms)
- **Complete isolation** from host system
- **Mounted access**: Only the session worktree
- **Customizable**: Use `--docker-image ubuntu:22.04` for specific environments

```bash
para start -p "analyze code" --container
```

### Security Considerations

- The `--dangerously-skip-permissions` flag bypasses IDE permission prompts - use with caution
- Choose your security level based on trust in the prompts and your environment
- Read access is intentionally broad across all sandbox levels (required for code analysis)
- Each level provides progressively stronger isolation at the cost of some functionality

That's it! Run `para config` to get started.
