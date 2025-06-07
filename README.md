# Para - Parallel IDE Workflow Helper

Work on multiple features simultaneously using Git worktrees and your favorite IDE.

## Installation

```bash
# Homebrew (recommended)
brew install 2mawi2/tap/para

# Or install directly
curl -sSL https://raw.githubusercontent.com/2mawi2/para/main/install-para.sh | bash
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
- `para continue` - Resume after resolving finish conflicts (auto-stages)
- `para cancel [session]` - Discard current or specified session
- `para cancel --group <name>` - Cancel all sessions in a multi-instance group
- `para clean` - Remove all sessions
- `para resume <session>` - Resume session in IDE

### Configuration
- `para config` - Interactive configuration wizard
- `para config auto` - Auto-detect IDE and create config
- `para config show` - Display current settings
- `para config edit` - Edit configuration file
- `para config quick` - Quick setup with confirmation

## AI Integration

Claude Code support for AI-powered development:
```bash
para dispatch "prompt"                    # Create session with AI prompt
para dispatch name "prompt"               # Named session with AI prompt
para dispatch --file prompt.txt          # Create session with prompt from file
para dispatch -f ./auth.prompt            # Create session with prompt from file (short form)
para dispatch-multi N "prompt"            # Create N parallel sessions with same prompt
para dispatch-multi N --file prompt.txt   # Create N sessions with prompt from file
para dispatch-multi N --group name "prompt"  # Create N sessions with custom group name

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

### Multi-Instance AI Development

The `dispatch-multi` command creates multiple parallel sessions with the same prompt, perfect for comparing different AI approaches:

```bash
# Create 3 sessions to explore different approaches
para dispatch-multi 3 "Implement user authentication system"

# Create 5 sessions with a custom group name for organization
para dispatch-multi 5 --group auth-experiments "Compare OAuth vs JWT implementation"

# List all sessions (shows group information)
para list

# Cancel individual sessions or entire groups
para cancel session-name              # Cancel one session
para cancel --group auth-experiments  # Cancel all sessions in group
```

The `dispatch` and `dispatch-multi` commands create new sessions and immediately open Claude Code with your prompt, perfect for AI-assisted development.

**Note:** Dispatch commands only work with Claude Code. Use `para config` to switch IDEs if needed.

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
├── subtrees/pc/             # Para sessions live here
│   ├── feature-auth-*/      # Session 1: authentication work
│   ├── feature-ui-*/        # Session 2: UI updates  
│   └── bugfix-login-*/      # Session 3: bug fix
└── .para_state/             # Para tracking (auto-managed)
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

### AI-Powered Multi-Instance Development
```bash
# Create multiple AI sessions to explore different approaches
para dispatch-multi 3 "Implement user authentication with best security practices"

# Or use a prompt file for complex prompts
para dispatch-multi 3 --file auth-requirements.prompt

# Each Claude Code instance works on the same prompt independently
# Compare results across 3 different approaches

# Pick the best implementation
para finish "Implement OAuth authentication"  # Keep this approach
para cancel --group multi-instance-group     # Discard other experiments

# Or finish multiple sessions with different messages
para finish "OAuth implementation - approach 1"
para finish "JWT implementation - approach 2"  
para finish "Session-based auth - approach 3"
```

## When Things Conflict

If two sessions modify the same files, you might get merge conflicts:

```bash
para finish "My changes"
# ❌ Conflicts detected → resolve manually, then:
para continue
# ✅ Finished!
```

## Perfect For AI Development

Para is ideal when working with AI assistants:

- **Multiple agents**: Each agent works in its own session without conflicts
- **Parallel experiments**: Use `dispatch-multi` to try different approaches simultaneously  
- **Safe iteration**: Main branch stays clean while you experiment
- **Easy comparison**: See results side-by-side in different IDE windows
- **A/B testing**: Compare multiple AI-generated solutions with identical prompts

## Documentation

- **[Detailed Configuration](docs/DETAILED_CONFIGURATION.md)** - Advanced IDE setup and environment variables
- **[Development Guide](docs/DEVELOPMENT.md)** - Contributing, architecture, testing
- **[Troubleshooting](docs/TROUBLESHOOTING.md)** - Common issues and solutions

## Requirements

- Git 2.5+ (for worktree support)
- Your preferred IDE with CLI support

## Environment Variables

Para supports several environment variables for configuration:

- **`IDE_NAME`** - IDE to use (`cursor`, `claude`, `code`, or custom)
- **`IDE_CMD`** - Command to launch the IDE
- **`IDE_USER_DATA_DIR`** - User data directory for IDE isolation
- **`BASE_BRANCH`** - Base branch for sessions (default: `main` or `master`)
- **`PARA_NON_INTERACTIVE`** - Skip interactive prompts (useful for CI/scripts)

### CI/Automation Usage

For automated environments, set `PARA_NON_INTERACTIVE=true` to skip welcome prompts:

```bash
export PARA_NON_INTERACTIVE=true
para start my-session
```

Para also auto-detects CI environments by checking for `CI` or `GITHUB_ACTIONS` environment variables.

## Shell Completion

Para supports tab completion for faster command-line usage. Enable it for your shell:

### Bash
```bash
# Save completion script to standard location
para completion generate bash > ~/.local/share/bash-completion/completions/para

# Or on some systems:
para completion generate bash > /usr/local/etc/bash_completion.d/para

# Restart your shell or source the completion
source ~/.local/share/bash-completion/completions/para
```

### Zsh
```bash
# Add to your fpath (create directory if needed)
mkdir -p ~/.local/share/zsh/site-functions
para completion generate zsh > ~/.local/share/zsh/site-functions/_para

# Add to your .zshrc if not already there:
echo 'fpath=(~/.local/share/zsh/site-functions $fpath)' >> ~/.zshrc
echo 'autoload -U compinit && compinit' >> ~/.zshrc

# Restart your shell
```

### Fish
```bash
# Save to fish completions directory (create if needed)
mkdir -p ~/.config/fish/completions
para completion generate fish > ~/.config/fish/completions/para.fish

# Restart your shell or reload completions
fish_update_completions
```

### What Gets Completed

Shell completion will help you with:
- **Commands**: `para [TAB]` shows all available commands
- **Session names**: `para cancel [TAB]` or `para resume [TAB]` shows active sessions
- **Group names**: `para cancel --group [TAB]` shows multi-instance groups
- **Branch names**: `para finish "msg" --branch [TAB]` shows local branches
- **File paths**: `para dispatch --file [TAB]` completes file paths

## Security Notes

The `--dangerously-skip-permissions` flag bypasses IDE permission warnings and should only be used in trusted environments like CI pipelines or automation scripts. It works with `start`, `dispatch`, and `dispatch-multi` commands.

**⚠️ Use with caution** - this flag may allow IDEs to access system resources without permission prompts.

That's it! Run `para config` to get started.