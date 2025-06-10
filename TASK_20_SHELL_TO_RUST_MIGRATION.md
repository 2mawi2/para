# TASK 20: Shell to Rust Migration

## Overview

Migrate from the current shell-based para implementation to the Rust implementation, making the Rust binary the primary para command. This is the foundational task that must be completed before CI/CD and distribution updates.

## Dependencies

**Prerequisites:** None - this is the foundation task
**Must complete before:** TASK 21, TASK 23, TASK 24

## Migration Strategy

### Phase 1: Preparation and Structure Changes

#### 1.1 Move Rust Implementation to Root
- Move `para-rs/` contents to root directory
- Integrate Rust build system with main justfile
- Update `.gitignore` to handle Rust target/ directory
- Remove shell implementation entirely (clean slate approach)

#### 1.2 Update Installation System
- Replace `install-para.sh` with Rust binary distribution via GitHub releases
- Update installation paths:
  - Binary: `~/.local/bin/para` (Rust executable)
  - Config: `~/.config/para/` (Rust JSON config)
- Use Homebrew as primary installation method

#### 1.3 Configuration System
- Use only Rust JSON configuration format
- No migration from shell config (fresh start)
- Configuration structure:
  - `ide.name` and `ide.command`
  - `directories.subtrees_dir`
  - Platform-specific defaults

### Phase 2: Build and Development Integration

#### 2.1 Update Justfile
- Integrate Rust build commands into main justfile
- Update test commands to run both Rust tests and legacy compatibility tests
- Add commands for:
  - `just build` - Build Rust binary
  - `just build-release` - Build optimized binary
  - `just install-rust` - Install Rust binary locally
  - `just test-all` - Run both Rust and legacy tests

#### 2.2 Development Workflow
- Update development setup commands
- Ensure `just dev-setup` installs Rust toolchain if needed
- Update linting to include Rust (clippy, rustfmt)
- Modify git hooks to run Rust checks

### Phase 3: Testing and Quality Assurance

#### 3.1 Rust Testing
- Focus on Rust unit and integration tests
- Remove legacy bats tests (not needed)
- Ensure comprehensive test coverage for all Rust functionality
- Test CLI interface and core functionality

#### 3.2 Shell Completion Implementation
- Implement shell completion using Rust clap_complete
- Generate completion scripts for bash, zsh, fish
- Ensure completion functions work with new binary
- Include completion in binary installation

### Phase 4: Deployment Preparation

#### 4.1 Prepare for CI/CD Integration
- Ensure Rust codebase is ready for GitHub Actions integration
- Verify Cargo.toml has correct package configuration
- Add any necessary metadata for releases
- Prepare for test workflow migration (handled in TASK 23)

#### 4.2 Prepare for Distribution
- Ensure binary builds correctly with `cargo build --release`
- Verify all CLI functionality works with compiled binary
- Prepare for Homebrew integration (handled in TASK 21)
- Document installation requirements

### Phase 5: Documentation and Cleanup

#### 5.1 Update Documentation
- Update README.md for new installation method
- Update CLAUDE.md to reflect Rust-first development
- Document new configuration system
- Remove references to shell implementation

#### 5.2 Clean Architecture
- Remove all shell scripts and lib/ directory
- Remove install-para.sh (replaced by Homebrew/releases)
- Remove legacy test files
- Clean up directory structure for Rust-only codebase

## Implementation Steps

### Directory Structure Changes
```
para/                          # Root directory
├── src/                       # Rust source (moved from para-rs/src/)
├── Cargo.toml                 # Rust manifest (moved from para-rs/)
├── target/                    # Rust build artifacts
├── docs/                      # Documentation
├── .github/workflows/         # Updated CI/CD
├── justfile                   # Integrated build system
└── README.md                  # Updated documentation
```

### Configuration System
```json
# Rust config (~/.config/para/config.json)
{
  "ide": {
    "name": "claude",
    "command": "claude"
  },
  "directories": {
    "subtrees_dir": "subtrees/pc"
  },
  "git": {
    "branch_prefix": "pc",
    "auto_stage": true
  }
}
```

### Build Integration
```bash
# Updated justfile commands
just build          # cargo build
just build-release   # cargo build --release
just test           # cargo test + clippy + fmt
just test-legacy    # Run bats tests against Rust binary
just install        # Build and install Rust binary
```

## Critical Requirements

### 1. Core Functionality
- All CLI commands must work as expected
- Clean Rust-only implementation
- No backwards compatibility requirements
- Fresh start with modern architecture

### 2. Performance Improvements
- Rust binary should be faster than shell script
- Startup time should be significantly reduced
- File operations should be more efficient
- macOS and Linux support only

### 3. Maintainability
- Single Rust codebase
- Type-safe configuration management
- Better error handling and user messages
- Comprehensive Rust test coverage

### 4. Distribution
- Source-based Homebrew formula using Cargo
- Automatic updates via GitHub Actions
- Clean installation process

## Testing Strategy

Ensure all Rust specific tests are running and comprehensive.

## Completion Criteria

- [ ] Rust binary replaces shell script as main para command
- [ ] All Rust tests pass  
- [ ] Shell completion works with Rust binary
- [ ] Documentation updated for new system
- [ ] Performance improvements demonstrated
- [ ] Clean codebase with no legacy components
- [ ] Cargo.toml configured for releases
- [ ] Binary builds and functions correctly
- [ ] Ready for CI/CD integration (TASK 23)
- [ ] Ready for Homebrew integration (TASK 21)

## Post-Migration Tasks

1. Remove all shell implementation files
2. Update documentation and tutorials
3. Announce new Rust implementation
4. Monitor for any issues and provide support

## Agent Instructions

1. **Test First**: Before making any changes, run all existing tests to ensure they pass
2. **Incremental Migration**: Make changes in small, testable increments
3. **Preserve Compatibility**: Never break existing functionality
4. **Document Changes**: Update documentation as you make changes
5. **Run Tests**: After each change, run both Rust tests and legacy compatibility tests
6. **Call `para finish 'Complete shell to Rust migration'`** when all tasks are completed and tests pass