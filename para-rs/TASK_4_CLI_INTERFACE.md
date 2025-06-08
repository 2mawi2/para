# Task 4: Enhanced CLI Interface

## Overview
Expand the basic CLI skeleton into a comprehensive command-line interface using clap v4 with all commands, arguments, and flags specified in the PRD.

## Scope
Build the `src/cli/` module with complete CLI functionality:

```
src/cli/
├── mod.rs           // Main CLI module interface
├── parser.rs        // Clap-based argument parsing and validation
├── commands/        // Individual command implementations
│   ├── mod.rs
│   ├── start.rs     // Start command
│   ├── dispatch.rs  // Dispatch command
│   ├── finish.rs    // Finish command
│   ├── integrate.rs // Integrate command
│   ├── cancel.rs    // Cancel command
│   ├── clean.rs     // Clean command
│   ├── list.rs      // List command
│   ├── resume.rs    // Resume command
│   ├── recover.rs   // Recover command
│   ├── continue.rs  // Continue command
│   └── config.rs    // Config command
└── completion.rs    // Shell completion generation
```

## Deliverables

### 1. Enhanced Argument Parser (`parser.rs`)
```rust
use clap::{Parser, Subcommand, Args, ValueEnum};

#[derive(Parser)]
#[command(name = "para")]
#[command(about = "Parallel IDE Workflow Helper")]
#[command(version, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create session with optional name
    Start(StartArgs),
    /// Start Claude Code session with prompt
    Dispatch(DispatchArgs),
    /// Squash all changes into single commit
    Finish(FinishArgs),
    /// Squash commits and merge into base branch
    Integrate(IntegrateArgs),
    /// Cancel session (moves to archive)
    Cancel(CancelArgs),
    /// Remove all active sessions
    Clean(CleanArgs),
    /// List active sessions
    #[command(alias = "ls")]
    List(ListArgs),
    /// Resume session in IDE
    Resume(ResumeArgs),
    /// Recover cancelled session from archive
    Recover(RecoverArgs),
    /// Complete merge after resolving conflicts
    Continue,
    /// Setup configuration
    Config(ConfigArgs),
    /// Generate shell completion script
    Completion(CompletionArgs),
}

// Detailed argument structures for each command
#[derive(Args)]
pub struct StartArgs {
    /// Session name (optional, generates friendly name if not provided)
    pub name: Option<String>,
    
    /// Skip IDE permission warnings (dangerous)
    #[arg(long, help = "Skip IDE permission warnings (dangerous)")]
    pub dangerously_skip_permissions: bool,
}

#[derive(Args)]
pub struct DispatchArgs {
    /// Session name or prompt text
    pub name_or_prompt: Option<String>,
    
    /// Additional prompt text (when first arg is session name)
    pub prompt: Option<String>,
    
    /// Read prompt from file
    #[arg(long, short = 'f', help = "Read prompt from specified file")]
    pub file: Option<PathBuf>,
    
    /// Skip IDE permission warnings (dangerous)
    #[arg(long, help = "Skip IDE permission warnings (dangerous)")]
    pub dangerously_skip_permissions: bool,
}

#[derive(Args)]
pub struct FinishArgs {
    /// Commit message
    pub message: String,
    
    /// Custom branch name after finishing
    #[arg(long, help = "Rename feature branch to specified name")]
    pub branch: Option<String>,
    
    /// Automatically integrate into base branch
    #[arg(long, short = 'i', help = "Automatically integrate into base branch")]
    pub integrate: bool,
    
    /// Session ID (optional, auto-detects if not provided)
    pub session: Option<String>,
}

// ... similar Args structs for other commands
```

### 2. Command Implementations (`commands/`)

Each command file should implement:
```rust
// Example: commands/start.rs
use crate::cli::parser::StartArgs;
use crate::utils::Result;

pub fn execute(args: StartArgs) -> Result<()> {
    // Input validation
    validate_start_args(&args)?;
    
    // Call core business logic (placeholder for now)
    println!("Start command would execute with args: {:?}", args);
    
    // For now, return "not implemented" error to maintain test compatibility
    Err(ParaError::not_implemented("start command"))
}

fn validate_start_args(args: &StartArgs) -> Result<()> {
    if let Some(ref name) = args.name {
        validate_session_name(name)?;
    }
    Ok(())
}
```

### 3. Advanced Argument Processing
```rust
// Auto-detection logic for dispatch command
impl DispatchArgs {
    pub fn resolve_prompt_and_session(&self) -> Result<(Option<String>, String)> {
        match (&self.name_or_prompt, &self.prompt, &self.file) {
            // --file flag provided
            (_, _, Some(file_path)) => {
                let prompt = read_file_content(file_path)?;
                Ok((self.name_or_prompt.clone(), prompt))
            }
            
            // Single argument - could be session name or prompt
            (Some(arg), None, None) => {
                if is_likely_file_path(arg) {
                    // Auto-detect file path
                    let prompt = read_file_content(Path::new(arg))?;
                    Ok((None, prompt))
                } else {
                    // Treat as prompt text
                    Ok((None, arg.clone()))
                }
            }
            
            // Two arguments - session name and prompt
            (Some(session), Some(prompt), None) => {
                Ok((Some(session.clone()), prompt.clone()))
            }
            
            _ => Err(ParaError::invalid_args("Invalid argument combination for dispatch")),
        }
    }
}

fn is_likely_file_path(input: &str) -> bool {
    input.contains('/') || 
    input.ends_with(".txt") || 
    input.ends_with(".md") || 
    input.ends_with(".prompt") ||
    Path::new(input).exists()
}
```

### 4. Shell Completion (`completion.rs`)
```rust
use clap_complete::{generate, Shell};
use clap_complete::shells::{Bash, Zsh, Fish};

pub fn generate_completion(shell: Shell) -> Result<String> {
    let mut cmd = crate::cli::parser::Cli::command();
    let mut buf = Vec::new();
    
    match shell {
        Shell::Bash => generate(Bash, &mut cmd, "para", &mut buf),
        Shell::Zsh => generate(Zsh, &mut cmd, "para", &mut buf),
        Shell::Fish => generate(Fish, &mut cmd, "para", &mut buf),
        _ => return Err(ParaError::invalid_args(format!("Unsupported shell: {:?}", shell))),
    }
    
    String::from_utf8(buf).map_err(|e| ParaError::invalid_args(format!("UTF-8 error: {}", e)))
}

// Enhanced completion with dynamic data
pub fn generate_enhanced_completion(shell: Shell) -> Result<String> {
    // TODO: Add dynamic completion for session names, branch names, etc.
    // This will integrate with other modules once they're implemented
    generate_completion(shell)
}
```

### 5. Main CLI Interface (`mod.rs`)
```rust
pub use parser::{Cli, Commands};
pub use completion::generate_completion;

// Main CLI execution function
pub fn execute_command(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Commands::Start(args)) => commands::start::execute(args),
        Some(Commands::Dispatch(args)) => commands::dispatch::execute(args),
        Some(Commands::Finish(args)) => commands::finish::execute(args),
        Some(Commands::Integrate(args)) => commands::integrate::execute(args),
        Some(Commands::Cancel(args)) => commands::cancel::execute(args),
        Some(Commands::Clean(args)) => commands::clean::execute(args),
        Some(Commands::List(args)) => commands::list::execute(args),
        Some(Commands::Resume(args)) => commands::resume::execute(args),
        Some(Commands::Recover(args)) => commands::recover::execute(args),
        Some(Commands::Continue) => commands::continue_cmd::execute(),
        Some(Commands::Config(args)) => commands::config::execute(args),
        Some(Commands::Completion(args)) => commands::completion::execute(args),
        None => {
            // Show usage when no command provided
            show_usage();
            Ok(())
        }
    }
}

fn show_usage() {
    println!("para - Parallel IDE Workflow Helper");
    // ... rest of usage text
}
```

### 6. Enhanced Validation and Help
```rust
// Custom validation for complex arguments
impl StartArgs {
    pub fn validate(&self) -> Result<()> {
        if let Some(ref name) = self.name {
            validate_session_name(name)?;
        }
        Ok(())
    }
}

// Custom help text and examples
impl Commands {
    pub fn examples(&self) -> &'static str {
        match self {
            Commands::Start(_) => {
                "Examples:\n  para start\n  para start my-feature\n  para start --dangerously-skip-permissions feature-auth"
            }
            Commands::Dispatch(_) => {
                "Examples:\n  para dispatch \"Add user authentication\"\n  para dispatch --file prompt.txt\n  para dispatch auth-feature --file requirements.md"
            }
            // ... other examples
        }
    }
}
```

## Dependencies
```toml
# Add to Cargo.toml
clap = { version = "4.5", features = ["derive", "color", "suggestions"] }
clap_complete = "4.5"
```

## Testing Approach
- Unit tests for argument parsing and validation
- Integration tests for command dispatch
- Test all argument combinations and edge cases
- Test shell completion generation
- Test error handling for invalid arguments
- Test help text and usage information

## Acceptance Criteria
✅ All commands from PRD are implemented with correct arguments  
✅ Advanced argument processing works (file auto-detection, etc.)  
✅ Shell completion works for bash, zsh, and fish  
✅ Validation provides helpful error messages  
✅ Help text is comprehensive and includes examples  
✅ All edge cases in argument parsing are handled  
✅ CLI maintains behavioral parity with shell version  
✅ Error messages guide users toward correct usage  
✅ Performance is acceptable for CLI responsiveness  

## Integration Points
- **Error types**: Uses error types from utils module for consistent error handling
- **Config**: Will call config module for `para config` command
- **Core modules**: Commands will delegate to appropriate core modules once implemented
- **File operations**: Uses file utilities for reading prompt files

## Command Specifications

### Session Creation Commands
- `para start [name] [--dangerously-skip-permissions]`
- `para dispatch [name] <prompt> [--file path] [--dangerously-skip-permissions]`

### Session Completion Commands  
- `para finish <message> [--branch name] [--integrate] [session]`
- `para integrate <message> [session]`
- `para continue`

### Session Management Commands
- `para list`
- `para cancel [session]`
- `para clean [--backups]`
- `para resume [session]`
- `para recover [session]`

### Configuration Commands
- `para config [subcommand]`
- `para completion generate <shell>`

## Notes
- This module provides the user interface but delegates business logic to core modules
- Focus on excellent user experience with helpful error messages and examples
- Maintain compatibility with existing shell version command syntax
- Implement comprehensive validation to catch user errors early
- Shell completion should be context-aware when possible