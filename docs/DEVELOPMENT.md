# Development Guide

## 🛠️ Development Setup

### Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Just command runner (if not already installed)
brew install just  # macOS
# or curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to ~/bin
```

### Setup Development Environment

```bash
# Build the project
cargo build

# Run tests  
just test
```

### Available Commands

```bash
just build          # Build debug binary
just build-release  # Build optimized release binary
just install        # Install Rust para binary globally
just test           # Run comprehensive Rust tests (formatting + tests + linting)
just test [filter]  # Run specific tests (e.g., just test finish)
just lint           # Run clippy linting
just fmt            # Auto-fix Rust formatting
just setup-hooks    # Configure git pre-commit/pre-push hooks
just status         # Show project status and dependencies
just clean          # Clean up development artifacts
```

### Git Hooks (Auto-configured)
- **Pre-commit**: Runs tests before commits
- **Pre-push**: Runs linting before pushes

### Testing
- Uses Rust's built-in test framework with `cargo test`
- Comprehensive unit and integration tests
- Git-isolated test environments using `src/test_utils.rs`
- Run with `just test` or `cargo test`

### Linting
- **clippy** for Rust static analysis
- **rustfmt** for consistent formatting  
- Run with `just lint` and `just fmt`

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
- **para-commands.sh**: Command handlers including dispatch with file input support
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
cd subtrees/para/*/                    # Enter worktree
echo 'test change' >> test-file.py   # Make changes
./para.sh finish "test commit"      # Auto-stage & finish
```

**Conflict Test:**
```bash
./para.sh start && ./para.sh start  # Create 2 sessions
cd subtrees/para/20*/                  # Session 1: modify same file
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