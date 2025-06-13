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
para completion generate bash > ~/.local/share/bash-completion/completions/para

# Create a new session (opens your IDE)
para start

# Work in the new IDE window...

# Merge your changes back to main
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
- `para integrate [session]` - Integrate session into main branch (with conflict handling)
- `para continue` - Resume after resolving integration conflicts (universal conflict detection)
- `para cancel [session]` - Discard current or specified session
- `para clean` - Remove all sessions
- `para resume <session>` - Resume session in IDE
- `para recover [session]` - Recover cancelled session from backup

### Configuration
- `para config` - Interactive configuration wizard
- `para config auto` - Auto-detect IDE and create config
- `para config show` - Display current settings
- `para config edit` - Edit configuration file
- `para config reset` - Reset configuration to defaults

## AI Integration

Claude Code support for AI-powered development:
```bash
para dispatch "prompt"                    # Create session with AI prompt
para dispatch name "prompt"               # Named session with AI prompt
para dispatch --file prompt.txt          # Create session with prompt from file
para dispatch -f ./auth.prompt            # Create session with prompt from file (short form)

# Skip permission warnings in trusted environments (CI, scripts)
para dispatch --dangerously-skip-permissions "prompt"
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

The `dispatch` command creates new sessions and immediately opens Claude Code with your prompt, perfect for AI-assisted development.

**Note:** Dispatch command only works with Claude Code. Use `para config` to switch IDEs if needed.

## Integration Strategies

Para supports multiple integration strategies for different development workflows:

```bash
# Rebase strategy (default) - clean linear history
para integrate --strategy rebase

# Merge strategy - preserves feature branch history  
para integrate --strategy merge

# Squash strategy - combines all commits into one
para integrate --strategy squash

# Preview integration without executing
para integrate --dry-run
```

### Integration Commands
- `para integrate [session]` - Integrate session with default strategy
- `para integrate --abort` - Abort active integration and restore original state
- `para continue` - Resume paused integration after conflict resolution

## How It Works

1. **Configure**: `para config` detects and configures your IDE (Cursor, Claude Code, VS Code, etc.)
2. **Create**: `para start` creates a timestamped Git branch and separate directory
3. **Work**: Each session gets its own IDE window and isolated workspace
4. **Merge**: `para finish` automatically stages changes and merges back to main
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
- **Cursor** 
- **VS Code**
- **Any IDE with CLI support**

```bash
para config              # Interactive setup wizard
para config auto         # Auto-detect IDE  
para config show         # Show current settings
para config edit         # Edit config file
```

### Configuration File Locations

Para stores configuration files in platform-specific locations:

**Shell Implementation:**
- `~/.config/para/config` (shell script format)

**Rust Implementation:**
- **macOS:** `~/Library/Application Support/para/config.json`
- **Linux:** `~/.config/para/config.json`
- **Windows:** `%APPDATA%\para\config.json`

The Rust implementation uses JSON format and is automatically created on first run.

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

# All features now merged to main branch
```

### AI-Powered Development
```bash
# Create AI session with prompt
para dispatch "Implement user authentication with best security practices"

# Or use a prompt file for complex prompts
para dispatch --file auth-requirements.prompt

# Claude Code opens with your prompt for AI-assisted development

# Finish when ready
para finish "Implement OAuth authentication"
```

## Integration & Conflict Resolution

Para provides robust conflict resolution with universal conflict detection for all Git operations:

### Basic Integration
```bash
# Integrate session into main branch
para integrate my-session
# ✅ Integration completed successfully!

# Or integrate current session
para integrate
```

### When Conflicts Occur
```bash
para integrate my-session
# ⚠️ Integration paused due to conflicts
# 📁 Conflicted files:
#    • README.md
#    • src/main.rs

# Resolve conflicts manually in your IDE, then:
para continue
# ✅ Integration completed successfully!
```

### Advanced Conflict Features
- **Universal conflict detection**: Works with merge, rebase, and cherry-pick operations
- **Automatic staging**: Resolved files are automatically staged
- **Smart operation detection**: Continues the appropriate Git operation (rebase/merge/cherry-pick)
- **State management**: Proper distinction between conflicts and failures
- **IDE integration**: Automatically opens conflicted files for resolution

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

**Bash:**
```bash
mkdir -p ~/.local/share/bash-completion/completions
para completion bash > ~/.local/share/bash-completion/completions/para
# Restart shell or run: source ~/.local/share/bash-completion/completions/para
```

**Zsh:**
```bash
mkdir -p ~/.local/share/zsh/site-functions
para completion zsh > ~/.local/share/zsh/site-functions/_para
echo 'fpath=(~/.local/share/zsh/site-functions $fpath)' >> ~/.zshrc
echo 'autoload -U compinit && compinit' >> ~/.zshrc
# Restart shell
```

**Fish:**
```bash
mkdir -p ~/.config/fish/completions  
para completion fish > ~/.config/fish/completions/para.fish
# Restart shell or run: fish_update_completions
```

### Smart Completions

The completion system provides intelligent suggestions:

**📁 Session Management:**
- `para resume <TAB>` → Shows active sessions
- `para cancel <TAB>` → Shows active sessions  
- `para recover <TAB>` → Shows archived sessions

**🌿 Branch & Integration:**
- `para integrate --target <TAB>` → Shows git branches
- `para integrate --strategy <TAB>` → Shows: `merge`, `squash`, `rebase`
- `para finish --branch <TAB>` → Shows git branches

**📄 File & Task Completion:**
- `para dispatch --file <TAB>` → Prioritizes TASK_*.md files and .md files
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

The `--dangerously-skip-permissions` flag bypasses IDE permission warnings and should only be used in trusted environments like CI pipelines or automation scripts. It works with `start` and `dispatch` commands.

**⚠️ Use with caution** - this flag may allow IDEs to access system resources without permission prompts.

That's it! Run `para config` to get started.
