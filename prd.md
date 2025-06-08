Of course. This is an excellent project for a Rust rewrite, offering improvements in performance, safety, and maintainability. Here is a complete Product Requirements Document (PRD) for rewriting the `para` codebase in Rust.

---

## Product Requirements Document: `para` Rewrite in Rust

### 1. Overview

**Project Name:** Para-rs

**Summary:** This document outlines the requirements for a complete rewrite of the `para` shell script utility into a compiled, high-performance, and cross-platform Rust application. The goal is to retain the full feature set and command-line interface (CLI) of the original `para` while gaining the benefits of Rust, such as type safety, performance, easier dependency management, and a single-binary distribution.

**Background:** The current `para` is a powerful and well-structured set of POSIX shell scripts that facilitates parallel development workflows using Git worktrees. While effective, its shell-based nature has limitations in performance, cross-platform compatibility (especially on Windows), and maintainability as complexity grows. A Rust rewrite will address these issues, providing a more robust and professional tool for developers.

**Key Mandate:** The rewrite must maintain behavioral parity with the original. To ensure this, the existing `bats` test suite will be leveraged to validate the new Rust binary.

### 2. Goals & Objectives

*   **Performance:** Significantly improve the speed of session management operations (start, finish, clean), especially filesystem and Git interactions.
*   **Reliability & Safety:** Eliminate entire classes of shell-scripting bugs (e.g., unexpected word splitting, unhandled errors) through Rust's strict compiler and type system.
*   **Maintainability:** Create a well-structured, modular Rust codebase that is easier to extend and maintain than the shell version.
*   **Cross-Platform Compatibility:** Produce a single, self-contained binary that runs consistently on macOS, Linux, and Windows (with Git installed).
*   **Behavioral Parity:** Ensure the Rust CLI is a drop-in replacement for the original `para.sh`, passing the existing `bats` test suite.
*   **Professional Tooling:** Leverage Rust's ecosystem for features like robust CLI parsing, error handling, and configuration management.

### 3. Target Audience

The primary users are software developers, particularly those who:
*   Work on multiple features or bug fixes simultaneously.
*   Engage in AI-assisted development and want to run multiple AI agents in parallel on the same codebase.
*   Value CLI-centric, efficient development workflows.
*   Work across different operating systems.

### 4. Proposed Architecture & Technical Design

The application will be a monolithic binary but will be internally structured into distinct, decoupled modules that mirror the logic of the original `lib/*.sh` files.

#### 4.1. Crate & Module Structure

**Updated Architecture for Enhanced Scalability:**

```
para-rs/
├── Cargo.toml
├── src/
│   ├── main.rs              // Entry point, minimal CLI dispatch
│   ├── cli/
│   │   ├── mod.rs           // CLI module organization
│   │   ├── parser.rs        // Clap-based argument parsing
│   │   ├── commands/        // Command implementations (one file per command)
│   │   │   ├── mod.rs
│   │   │   ├── start.rs
│   │   │   ├── dispatch.rs
│   │   │   ├── finish.rs
│   │   │   ├── integrate.rs
│   │   │   ├── cancel.rs
│   │   │   ├── clean.rs
│   │   │   ├── list.rs
│   │   │   ├── resume.rs
│   │   │   ├── recover.rs
│   │   │   ├── continue.rs
│   │   │   └── config.rs
│   │   └── completion.rs    // Shell completion generation
│   ├── core/                // Core business logic (reusable)
│   │   ├── mod.rs
│   │   ├── session/         // Session management
│   │   │   ├── mod.rs
│   │   │   ├── lifecycle.rs  // Create, finish, cancel operations
│   │   │   ├── state.rs     // Session state management
│   │   │   ├── recovery.rs  // Archive and backup recovery
│   │   │   └── validation.rs // Session name validation, etc.
│   │   ├── git/             // Git operations
│   │   │   ├── mod.rs
│   │   │   ├── worktree.rs  // Worktree creation/removal
│   │   │   ├── branch.rs    // Branch operations
│   │   │   ├── integration.rs // Rebase and merge logic
│   │   │   └── repository.rs // Repository discovery and validation
│   │   ├── ide/             // IDE integration
│   │   │   ├── mod.rs
│   │   │   ├── launcher.rs  // IDE launching logic
│   │   │   ├── wrapper.rs   // Wrapper mode support
│   │   │   ├── tasks.rs     // VS Code/Cursor task generation
│   │   │   └── process.rs   // Process management and window closing
│   │   └── storage/         // File system operations
│   │       ├── mod.rs
│   │       ├── state.rs     // State file management
│   │       ├── backup.rs    // Backup system
│   │       └── paths.rs     // Path resolution and validation
│   ├── config/              // Configuration management
│   │   ├── mod.rs
│   │   ├── manager.rs       // Configuration loading/saving
│   │   ├── wizard.rs        // Interactive configuration setup
│   │   ├── validation.rs    // Configuration validation
│   │   └── defaults.rs      // Default values and IDE detection
│   ├── utils/               // Utilities and helpers
│   │   ├── mod.rs
│   │   ├── error.rs         // Error types and handling
│   │   ├── fs.rs           // File system utilities
│   │   ├── names.rs        // Name generation (friendly names, etc.)
│   │   └── json.rs         // JSON handling utilities
│   └── platform/            // Platform-specific code
│       ├── mod.rs
│       ├── macos.rs        // macOS-specific IDE closing, etc.
│       ├── linux.rs        // Linux-specific implementations
│       └── windows.rs      // Windows-specific implementations
├── tests/
│   ├── integration/         // Integration tests
│   │   ├── mod.rs
│   │   ├── session_lifecycle.rs
│   │   ├── git_operations.rs
│   │   └── ide_integration.rs
│   ├── unit/               // Unit tests
│   │   └── ...
│   └── bats_runner.rs      // Rust test that runs existing bats tests
└── benches/                // Benchmarks
    └── session_performance.rs
```

#### 4.2. Key Architectural Decisions

1.  **Git Interaction:** Instead of using Rust's `git2-rs` library, we will **use `std::process::Command` to call the `git` command-line executable directly**.
    *   **Reasoning:** This approach guarantees 100% compatibility with the user's existing Git setup and behavior. It avoids the immense complexity of re-implementing `git`'s nuanced logic (worktrees, rebase, etc.) with `libgit2`, which could introduce subtle bugs. This also simplifies achieving behavioral parity for the existing tests.

2.  **State Management:** Session state will continue to be stored in the file system (`.para_state/`) within the project repository to maintain the original's stateless design. We will use strongly-typed structs and `serde` for reading/writing state files, making the process more robust.

3.  **Modular Architecture:** The crate is organized into layers with clear separation of concerns:
    *   **CLI Layer:** Handles user interaction and argument parsing only
    *   **Core Layer:** Contains all business logic, fully testable without CLI dependencies
    *   **Platform Layer:** Isolates OS-specific code for maximum portability

4.  **Trait-Based Design:** Core functionality will be implemented using traits to enable:
    *   Easy unit testing with mock implementations
    *   Future extensibility for new IDEs or storage backends
    *   Clean dependency injection and inversion of control

5.  **Comprehensive Error Handling:** Custom error types using `thiserror` will provide structured, actionable error messages with proper error chaining and context.

#### 4.3. Recommended Libraries

| Category              | Library                                        | Rationale                                                                                                  |
| --------------------- | ---------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| **CLI Parsing**       | `clap` (v4, with derive feature)               | The de-facto standard for building powerful, fast, and correct CLIs. Will generate shell completions natively. |
| **Configuration**     | `config-rs`, `serde`                           | Easily manage configuration from files and environment variables. `serde` provides robust serialization.     |
| **Directory Paths**   | `directories-rs`                               | Provides cross-platform, standard-compliant access to config/data directories (XDG).                     |
| **Error Handling**    | `anyhow` and `thiserror`                       | `anyhow` for simple, flexible error handling in the application; `thiserror` for creating custom error types. |
| **Terminal Output**   | `colored` or `console`                         | For user-friendly, colored output (e.g., for warnings, success messages).                                  |
| **Interactive Prompts**| `dialoguer`                                    | For implementing the interactive `para config` wizard.                                                     |
| **JSON Handling**     | `serde_json`                                   | For generating the `.vscode/tasks.json` file used by the IDE wrapper mode.                                 |

#### 4.4. Architectural Principles

**1. Separation of Concerns:**
*   CLI layer handles only user interaction and argument parsing
*   Core layer contains all business logic, fully testable without CLI dependencies  
*   Platform layer isolates OS-specific implementations

**2. Trait-Based Design for Extensibility:**
```rust
trait GitRepository {
    fn create_worktree(&self, branch: &str, path: &Path) -> Result<()>;
    fn remove_worktree(&self, path: &Path) -> Result<()>;
}

trait IdeManager {
    fn launch(&self, ide: &IdeConfig, path: &Path, prompt: Option<&str>) -> Result<()>;
    fn close_session(&self, session_id: &str) -> Result<()>;
}

trait SessionStorage {
    fn save_session(&self, session: &Session) -> Result<()>;
    fn load_session(&self, id: &str) -> Result<Session>;
    fn list_sessions(&self) -> Result<Vec<SessionSummary>>;
}
```

**3. Comprehensive Error Handling:**
```rust
#[derive(thiserror::Error, Debug)]
pub enum ParaError {
    #[error("Git operation failed: {message}")]
    GitOperation { message: String },
    
    #[error("Session '{session_id}' not found")]
    SessionNotFound { session_id: String },
    
    #[error("Configuration error: {message}")]
    Config { message: String },
    
    #[error("IDE error: {message}")]
    Ide { message: String },
    
    #[error("File system error: {0}")]
    FileSystem(#[from] std::io::Error),
}
```

**4. Future-Ready Design:**
*   Async-ready patterns even for initially synchronous operations
*   Plugin architecture foundation for future IDE extensions
*   Performance optimization hooks for benchmarking and monitoring

### 5. Feature Requirements & CLI Interactions

This section details the required commands. All commands must be implemented as subcommands in `clap`.

**Note:** This PRD includes enhanced features from the current shell implementation that provide significant value and should be preserved in the Rust rewrite.

#### 5.1. Session Creation

**Commands:** `start`, `dispatch`

*   **`para start [NAME]`**
    *   **CLI:** An optional `NAME` argument for the session. If omitted, a friendly, unique name (e.g., `agile_phoenix_20231027-103000`) should be generated.
    *   **Behavior:**
        1.  Validate that the current directory is a Git repository.
        2.  Check for uncommitted changes and warn the user (but do not block).
        3.  Generate a unique branch name and worktree path.
        4.  Execute `git worktree add -b <branch> <path> HEAD`.
        5.  Create a session state file in `.para_state/` containing branch, path, and base branch info.
        6.  Launch the configured IDE, pointing it to the new worktree path.
    *   **Flags:**
        *   `--dangerously-skip-permissions`: (Boolean flag) A pass-through flag for the IDE launcher.

*   **`para dispatch [NAME] <PROMPT>`**
    *   **CLI:** An optional `NAME` and a required `PROMPT` string.
    *   **Behavior:**
        1.  Identical to `para start`, but with an additional step.
        2.  The `PROMPT` is passed to the IDE launcher.
        3.  For `claude`, this involves generating a `.vscode/tasks.json` that executes `claude` with the prompt.
        4.  This command should fail if the configured IDE is not `claude`.
        5.  **Auto-detection:** If a single argument is provided and it looks like a file path (contains `/` or common extensions like `.txt`, `.md`, `.prompt`), automatically treat it as a file path rather than requiring the `--file` flag.
    *   **Flags:**
        *   `--file <PATH>`, `-f <PATH>`: Reads the prompt from the specified file instead of a command-line argument.
        *   `--dangerously-skip-permissions`: Same as `start`.

#### 5.2. Session Completion

**Commands:** `finish`, `integrate`, `continue`

*   **`para finish <MESSAGE>`**
    *   **CLI:** A required `MESSAGE` string for the commit.
    *   **Behavior:**
        1.  Auto-detect the current session from the working directory. If ambiguous, error and list sessions.
        2.  Change directory to the session's worktree.
        3.  Check if there are any new commits since the base branch.
        4.  If multiple new commits exist, perform a `git reset --soft <base-branch>` to squash them.
        5.  Stage all changes (`git add -A`).
        6.  Create a single new commit using the provided `MESSAGE`.
        7.  Clean up the session: remove the worktree directory and the session state file.
        8.  The feature branch remains for manual merging or PR creation.
        9.  Print a success message indicating the branch name to merge.
    *   **Flags:**
        *   `--branch <BRANCH_NAME>`: After squashing, rename the feature branch to `<BRANCH_NAME>`. Handle conflicts if the name already exists by appending a suffix (e.g., `-1`).
        *   `--integrate`, `-i`: After a successful finish, automatically attempt to merge the feature branch into the base branch (see `integrate`).

*   **`para integrate <MESSAGE>`**
    *   **CLI:** A required `MESSAGE` string.
    *   **Behavior:**
        1.  Identical to `para finish "message" --integrate`.
        2.  After the commit is created on the feature branch, it will:
            a. Switch to the base branch (e.g., `main`).
            b. Pull the latest changes from the remote, if one is configured.
            c. Attempt to `rebase` the feature branch onto the updated base branch.
            d. If the rebase is successful, fast-forward merge the rebased changes into the base branch.
            e. Delete the feature branch and clean up the session.
            f. If rebase fails due to conflicts, save the conflict state and instruct the user to resolve conflicts and run `para continue`.

*   **`para continue`**
    *   **CLI:** No arguments.
    *   **Behavior:**
        1.  Check for a saved integration conflict state. If none, error.
        2.  Check if Git conflicts are resolved. If not, error and list conflicting files.
        3.  Execute `git rebase --continue`.
        4.  On success, complete the merge into the base branch, clean up the feature branch, and clear the conflict state.

#### 5.3. Session Management & State

**Commands:** `list`, `cancel`, `clean`, `resume`, `recover`

*   **`para list` (alias `ls`)**
    *   **CLI:** No arguments.
    *   **Behavior:** Read all files in `.para_state/`, display a formatted list of active sessions, including their name, branch, worktree path, and status (Clean, Modified, Staged).

*   **`para cancel [SESSION]`**
    *   **CLI:** An optional `SESSION` name. If omitted, auto-detects the current session.
    *   **Behavior:**
        1.  Remove the worktree directory.
        2.  **Move the Git branch** to an archive namespace (e.g., `para/my-feature` -> `para/archive/my-feature`). This preserves the work for recovery.
        3.  Delete the session state file.

*   **`para clean`**
    *   **CLI:** No arguments.
    *   **Behavior:** Iterates through all active sessions and performs the `cancel` action on them, but moves branches to the archive.
    *   **Flags:**
        *   `--backups`: Deletes all branches from the `para/archive/` namespace, permanently removing cancelled session backups. This also cleans up the backup system files for cancelled sessions.

*   **`para resume [SESSION]`**
    *   **CLI:** An optional `SESSION` name. If omitted, and only one session exists, resume it. If multiple exist, list them and prompt the user.
    *   **Behavior:** 
        1.  Launch the configured IDE for the specified session's worktree.
        2.  **Enhanced discovery:** Can resume orphaned worktrees (directories that exist but lack state files) with limited functionality.
        3.  **Smart session detection:** Auto-detect sessions from current working directory when possible.

*   **`para recover [SESSION]`**
    *   **CLI:** An optional `SESSION` name.
    *   **Behavior:**
        1.  If no name is provided, list all branches in the `para/archive/` namespace.
        2.  If a name is provided, find the corresponding archived branch, move it back to the active namespace (`para/`), recreate the worktree, and create a new session state file.

#### 5.4. Configuration & Meta-Commands

**Commands:** `config`, `completion`, `--version`, `--help`

*   **`para config [subcommand]`**
    *   **CLI:** A subcommand-based interface.
    *   **Behavior:**
        *   `para config`: An interactive wizard (using `dialoguer`) to set IDE and other options.
        *   `show`: Display the current configuration.
        *   `edit`: Open the config file in `$EDITOR`.
        *   `auto`/`quick`: Auto-detect installed IDEs and create a default config.

*   **`para completion generate <SHELL>`**
    *   **CLI:** A required `SHELL` argument (`bash`, `zsh`, `fish`).
    *   **Enhanced completion features:**
        *   Context-aware completion for session names in `cancel`, `resume`, `recover` commands
        *   Branch name completion for `finish --branch` flag
        *   File path completion for `dispatch --file` flag
    *   **Behavior:** `clap` will generate the appropriate shell completion script, which is then printed to standard output.

*   **`--version`/`-v` and `--help`/`-h`**: Standard flags handled by `clap`.

#### 5.5. Enhanced Features from Current Implementation

**Advanced IDE Integration:**
*   **IDE Wrapper Mode:** Support for launching Claude Code inside VS Code or Cursor wrapper IDEs
*   **Automatic IDE Window Closing:** Platform-specific window management (macOS AppleScript support)
*   **Conflict Resolution Integration:** Automatically open IDE with conflicted files during rebase conflicts

**Dual Recovery System:**
*   **Git Archive Recovery:** Primary recovery via `para/archive/` namespace branches  
*   **Backup System:** Secondary backup system for the last 3 cancelled sessions with metadata

**Configuration Flexibility:**
*   **Configurable Branch Prefixes:** Allow custom prefixes instead of hardcoded "para"
*   **IDE-Specific Configuration:** Per-IDE settings for command paths, user data directories, etc.
*   **Environment Variable Overrides:** Support for runtime configuration via environment variables

**Enhanced User Experience:**
*   **Smart Auto-Detection:** Automatically detect session context from current working directory
*   **Friendly Session Names:** Generate human-readable session names (e.g., `clever_phoenix_20231027-143022`)
*   **Orphaned Worktree Support:** Handle and resume worktrees that lost their state files

### 6. Testing & Validation Strategy

The core requirement is to reuse the existing `bats` test suite. This ensures high-fidelity behavioral compatibility.

#### 6.1. The `bats` Test Runner

A new test runner script (`tests/bats_runner.sh`) will be created to orchestrate testing the Rust binary.

**Execution Flow:**
1.  **Compile:** The script will first run `cargo build` to compile the `para` Rust binary (e.g., to `target/debug/para`).
2.  **Shim `para.sh`:** The original `para.sh` in the project root will be temporarily replaced with a shim or a symlink that points to the compiled Rust binary. This is the crucial step that redirects the test suite's calls.
    ```bash
    # In bats_runner.sh
    RUST_BINARY_PATH="$(pwd)/target/debug/para"
    # Temporarily replace para.sh with a script that calls the Rust binary
    mv para.sh para.sh.bak
    echo "#!/bin/sh" > para.sh
    echo "exec \"$RUST_BINARY_PATH\" \"\$@\"" >> para.sh
    chmod +x para.sh
    ```
3.  **Execute Tests:** The script will then execute the `bats` suite: `bats tests/`.
4.  **Cleanup:** After the tests complete, the runner will restore the original `para.sh` and remove the shim.

This process will be integrated into the CI pipeline (`.github/workflows/test.yml`) and can be run locally via `just test`.

#### 6.2. Unit and Integration Tests in Rust

While the `bats` suite covers black-box testing, Rust's native testing capabilities should also be used for:
*   **Unit Tests:** Testing individual functions within modules (e.g., `config.rs`, `utils.rs`) for correctness.
*   **Integration Tests:** Testing interactions between modules in isolation (e.g., ensuring the `session` module correctly calls the `git` module).

### 7. Non-Functional Requirements

*   **Performance:** Session creation (`start`) and cleanup (`cancel`/`finish`) should complete in under 500ms on a modern SSD for a small repository, a significant improvement over shell script startup times.
*   **Security:**
    *   The application must not require elevated privileges.
    *   Input sanitization should be performed on all user-provided strings that are used in shell commands (e.g., commit messages, branch names).
    *   The `--dangerously-skip-permissions` flag must be clearly documented as a security risk.
*   **Usability:**
    *   CLI output must be clear, concise, and user-friendly. Use color to differentiate errors, warnings, and success messages.
    *   Error messages must be actionable, guiding the user on how to resolve the issue.
    *   Shell completions are essential for a smooth user experience.

### 8. Out of Scope

*   **Changing Core Logic:** This is a rewrite, not a redesign. The fundamental workflow of using Git worktrees will remain unchanged.
*   **GUI Interface:** This will remain a CLI-only tool.
*   **Removing Git Dependency:** The tool will still require `git` to be installed and available in the system's `PATH`.