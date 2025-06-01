# Para - Parallel IDE Workflow Helper

Work on multiple features simultaneously using Git worktrees and your favorite IDE.

## Why Para?

**Problem:** You want to work on multiple features, experiments, or bug fixes at the same time, but switching Git branches disrupts your workflow and mixes up uncommitted changes.

**Solution:** Para creates isolated development environments (separate directories + Git branches) so you can:
- Have multiple IDE windows open, each working on different features
- Switch instantly between projects without losing context
- Let AI agents work in parallel without interfering with each other
- Keep your main branch always clean

**Perfect for:** AI-assisted development, feature prototyping, parallel experiments, or any workflow where you need multiple isolated workspaces.

## Quick Start

```bash
# Install
./install-para.sh

# Configure your IDE (one-time setup)
para config

# Create a new session (opens your IDE)
para start

# Work in the new IDE window...

# Merge your changes back to main
para finish "Add new feature"
```

## Core Commands

```bash
para config              # Set up your IDE (first-time setup)
para start [name]        # Create new session
para list               # Show active sessions  
para finish "message"   # Merge session back to main
para cancel             # Delete current session
para clean              # Delete all sessions
```

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

## Example Workflow

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
- **Parallel experiments**: Try different approaches simultaneously  
- **Safe iteration**: Main branch stays clean while you experiment
- **Easy comparison**: See results side-by-side in different IDE windows

## Installation

```bash
# Download and run the installer
curl -sSL https://raw.githubusercontent.com/your-repo/para/main/install-para.sh | bash

# Or clone and install locally  
git clone https://github.com/your-repo/para
cd para
./install-para.sh
```

## Documentation

- **[Detailed Configuration](docs/DETAILED_CONFIGURATION.md)** - Advanced IDE setup and environment variables
- **[Development Guide](docs/DEVELOPMENT.md)** - Contributing, architecture, testing
- **[Troubleshooting](docs/TROUBLESHOOTING.md)** - Common issues and solutions

## Requirements

- Git 2.5+ (for worktree support)
- Your preferred IDE with CLI support

That's it! Run `para config` to get started.