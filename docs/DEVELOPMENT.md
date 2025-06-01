# Development Guide

## 🛠️ Development Setup

### Prerequisites

```bash
# Install Just command runner (if not already installed)
brew install just  # macOS
# or curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to ~/bin
```

### Setup Development Environment

```bash
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

### Module Structure

Each module in `lib/` has a specific responsibility:

- **para-config.sh**: Environment setup and configuration loading
- **para-utils.sh**: Common utilities, validation, and helper functions
- **para-git.sh**: All Git operations including worktree and finish management
- **para-session.sh**: Session lifecycle, state management, and detection
- **para-ide.sh**: IDE integration with extensible interface

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

## 🧪 Testing Workflows

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

## 🤝 Contributing

The modular architecture makes contributions easier:

1. **Bug fixes**: Usually isolated to a single module
2. **New features**: Can often be added by extending existing modules
3. **IDE support**: Add new implementations to `para-ide.sh`
4. **Git workflows**: Extend `para-git.sh` for new finish strategies 