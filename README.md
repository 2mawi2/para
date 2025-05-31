# Pursor - Parallel-Cursor Workflow Helper

A simple script to create multiple ephemeral Cursor IDE sessions on temporary Git worktrees, then merge or discard changes with a single command.

Perfect for prototyping multiple features simultaneously while keeping your main branch clean.

## 🚀 Quick Start

```bash
# Create a new parallel session
pursor

# Work in the new Cursor window that opens...

# Merge your changes back
pursor merge "Add new feature"
```

## 📦 Installation

### Universal Installer (Recommended)

The universal installer works with **any shell** (bash, zsh, fish, sh, etc.):

```bash
# Download and run the universal installer
./install-pursor.sh
```

This will:
- Auto-detect your shell (bash/zsh/fish/other)
- Install `pursor` to `~/.local/bin/pursor`
- Add `~/.local/bin` to your PATH in the appropriate config file
- Work immediately after installation

### Alternative: Shell-Specific Installer

If you prefer a shell-specific approach, the fish installer is available as an example:

```bash
# For fish users (example of shell-specific installer)
./install-pursor.fish
```

### Manual Installation

1. Copy `pursor.sh` to your Git repository
2. Make it executable: `chmod +x pursor.sh`
3. Optionally, copy to `~/.local/bin/pursor` for global access

**Shell Compatibility**: Pursor is written in POSIX shell and works with all major shells including bash, zsh, fish, dash, and ash.

## 🎯 Usage

### Basic Commands

```bash
pursor                    # Create new session → opens Cursor
pursor list               # List all active sessions (alias: ls)
pursor merge "message"    # Merge session back to main
pursor cancel             # Cancel/delete session (alias: abort)
pursor clean              # Cancel ALL sessions (clean everything)
```

### Merge Strategies

Choose how to integrate your changes back into the main branch:

```bash
# Default: Rebase strategy (clean linear history)
pursor merge "Add feature"

# Explicit strategies
pursor merge "Add feature" --strategy=rebase      # Clean linear history (default)
pursor merge "Add feature" --strategy=merge       # Preserve branch structure
pursor merge "Add feature" --strategy=squash      # Single commit with all changes
pursor merge "Hotfix" --strategy=cherry-pick      # Apply specific commits only
pursor merge "Feature" --strategy=branch          # Create permanent feature branch
```

#### Strategy Details

**🔄 Rebase (default)**
- Rebases your session commits onto the latest main branch
- Creates clean, linear history
- Best for: Most development work, clean commit history
- Handles conflicts during rebase phase

**🌿 Merge**
- Direct merge without rebasing
- Preserves branch structure in git history
- Best for: When you want to preserve the parallel development story
- Handles conflicts during merge phase

**📦 Squash**
- Combines all session commits into a single commit
- Clean, single commit with all changes
- Best for: Feature completion, cleanup commits, experimental work
- Perfect commit message describes the entire feature

**🍒 Cherry-pick**
- Applies only committed changes as individual commits
- Skips any uncommitted changes in the worktree
- Best for: Applying specific fixes, selective integration
- Each commit maintains its original message

**🌳 Branch**
- Creates a permanent feature branch instead of merging
- Preserves all work for later integration
- Best for: Work-in-progress, features needing review, experimental branches
- Allows for standard git workflows (pull requests, etc.)

#### Strategy Examples

```bash
# Clean feature development
pursor merge "Implement user authentication" --strategy=rebase

# Complex feature with multiple developers
pursor merge "Payment system integration" --strategy=merge

# Cleanup experimental work into single commit  
pursor merge "Refactor data layer" --strategy=squash

# Apply only the critical bugfix commits
pursor merge "Fix security issues" --strategy=cherry-pick

# Create feature branch for code review
pursor merge "New dashboard UI" --strategy=branch
# → Creates feature/new-dashboard-ui branch
# Later: git checkout main && git merge feature/new-dashboard-ui
```

### Multi-Session Workflow

```bash
# Create multiple sessions
pursor                    # Session 1 (opens Cursor)
pursor                    # Session 2 (opens Cursor) 

# List active sessions
pursor list
# Session: pc-20250531-143022
#   Branch: pc/20250531-143022
#   Status: Clean
# Session: pc-20250531-143025  
#   Branch: pc/20250531-143025
#   Status: Has uncommitted changes

# Merge sessions (auto-detects from current directory!)
cd subtrees/pc/20250531-143022
pursor merge "Feature A complete"

cd ../20250531-143025
pursor merge "Feature B complete"

# Or cancel individual sessions
pursor cancel

# Or cancel ALL sessions at once
pursor clean
# ✅ cleaned up 3 session(s)
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
# ❌ rebase conflicts in session pc-20250531-143022
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

- **Session Creation**: Creates `pc/YYYYMMDD-HHMMSS` branch and `subtrees/pc/YYYYMMDD-HHMMSS` worktree
- **State Tracking**: Uses `.pursor_state/` directory to track active sessions
- **Auto-Staging**: Automatically stages all changes during merge and conflict resolution
- **Context-Aware**: Auto-detects which session you're working on from your current directory
- **Clean Merging**: Attempts fast-forward, falls back to merge commit if needed
- **Auto-Cleanup**: Removes worktrees, branches, and state files after successful merge

## 💡 Tips

- **Context Detection**: When working in a session directory, commands auto-detect the session
- **No Manual Git**: Never need to run `git add` - everything is auto-staged
- **Conflict Prevention**: Keep sessions focused on different areas of the codebase
- **Session Naming**: Session IDs are auto-generated timestamps for uniqueness
- **Cursor Integration**: Sessions automatically open in new Cursor windows
- **Run Anywhere**: Script works from any directory in the repository, including from within subtrees

## 🌐 Global Usage

The script automatically detects the repository root and current session context:

```bash
# From main repository directory
pursor merge "Feature complete"

# From within a subtree/worktree (auto-detects session!)
cd subtrees/pc/20250531-143022
pursor merge "Feature complete"    # Knows you mean this session
pursor continue                    # Resumes this session after conflicts
pursor cancel                      # Cancels this session

# From any subdirectory
cd src/components
pursor list                        # Shows all sessions
```

This makes it perfect for seamless workflow entirely within subtrees.

## 🛠 Configuration

Set environment variables to customize behavior:

```bash
export BASE_BRANCH="develop"                    # Base branch (default: current branch)
export SUBTREES_DIR_NAME="worktrees"           # Directory name (default: subtrees)
export STATE_DIR_NAME=".my_state"              # State directory (default: .pursor_state)
export CURSOR_CMD="code"                       # Editor command (default: cursor)
export DEFAULT_MERGE_STRATEGY="squash"         # Default merge strategy (default: rebase)
```

## 🎯 Perfect For

- ✅ Prototyping multiple approaches
- ✅ Working on independent features simultaneously  
- ✅ Testing different implementations
- ✅ Keeping experimental work isolated
- ✅ Quick feature comparisons
- ✅ Seamless conflict resolution workflow

## 🔥 Advanced Workflow

```bash
# Start multiple parallel experiments
pursor                              # Feature A
pursor                              # Feature B  
pursor                              # Feature C

# Work on Feature A
cd subtrees/pc/20250531-120001
# ... make changes ...
pursor merge "Implement approach A"  # Auto-stages & merges

# Work on Feature B (with conflicts)
cd ../20250531-120002
# ... make changes ...
pursor merge "Implement approach B"  # Conflict occurs!
# ... fix conflicts in editor ...
pursor continue                     # Auto-stages resolved files & completes

# Cancel Feature C (not working out)
cd ../20250531-120003
pursor cancel                       # Clean removal

# All done - verify clean state
cd ../../..
pursor list                         # "No active parallel sessions."
```

---

**No external dependencies • Pure POSIX shell • Works locally • Context-aware**