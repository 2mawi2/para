use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "para")]
#[command(about = "Parallel IDE Workflow Helper")]
#[command(
    version,
    long_about = "When run without any command, opens the monitor view to manage active sessions"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create session with optional name
    Start(StartArgs),
    /// Start Claude Code session with prompt (supports stdin piping)
    Dispatch(DispatchArgs),
    /// Squash all changes into single commit
    Finish(FinishArgs),
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
    /// Setup configuration
    Config(ConfigArgs),
    /// Generate shell completion script
    Completion(CompletionArgs),
    /// Initialize shell completions automatically
    Init,
    /// Setup Model Context Protocol (MCP) integration
    Mcp(crate::cli::commands::mcp::McpCommand),
    /// Legacy completion endpoint for sessions (hidden)
    #[command(name = "_completion_sessions", hide = true)]
    CompletionSessions,
    /// Legacy completion endpoint for branches (hidden)
    #[command(name = "_completion_branches", hide = true)]
    CompletionBranches,
    /// Monitor and manage active sessions in real-time (interactive TUI with mouse support)
    Monitor(MonitorArgs),
    /// Update session status (for agents to communicate progress)
    Status(StatusArgs),
}

#[derive(Args, Debug)]
pub struct StartArgs {
    /// Session name (optional, generates friendly name if not provided)
    pub name: Option<String>,

    /// Skip IDE permission warnings (dangerous)
    #[arg(long, help = "Skip IDE permission warnings (dangerous)")]
    pub dangerously_skip_permissions: bool,
}

#[derive(Args, Debug)]
#[command(after_help = "EXAMPLES:
    # Basic usage with inline prompt
    para dispatch \"implement user authentication\"
    
    # With custom session name
    para dispatch auth-feature \"implement user authentication\"
    
    # From file
    para dispatch --file prompt.txt
    para dispatch auth-feature --file requirements.md
    
    # Using stdin piping
    echo \"test prompt\" | para dispatch
    cat requirements.txt | para dispatch my-feature
    jq '.description' task.json | para dispatch")]
pub struct DispatchArgs {
    /// Session name or prompt text
    pub name_or_prompt: Option<String>,

    /// Additional prompt text (when first arg is session name)
    pub prompt: Option<String>,

    /// Read prompt from file
    #[arg(long, short = 'f', help = "Read prompt from specified file")]
    pub file: Option<PathBuf>,

    /// Skip IDE permission warnings (dangerous)
    #[arg(long, short = 'd', help = "Skip IDE permission warnings (dangerous)")]
    pub dangerously_skip_permissions: bool,
}

#[derive(Args, Debug)]
pub struct FinishArgs {
    /// Commit message
    pub message: String,

    /// Custom branch name after finishing
    #[arg(long, short = 'b', help = "Rename feature branch to specified name")]
    pub branch: Option<String>,

    /// Session ID (optional, auto-detects if not provided)
    pub session: Option<String>,
}

#[derive(Args, Debug)]
pub struct CancelArgs {
    /// Session ID (optional, auto-detects if not provided)
    pub session: Option<String>,

    /// Force cancellation even with uncommitted changes (destructive)
    #[arg(
        long,
        short,
        help = "Force cancellation even with uncommitted changes (destructive)"
    )]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct CleanArgs {
    /// Skip confirmation prompts
    #[arg(long, short, help = "Skip confirmation prompts")]
    pub force: bool,

    /// Only show what would be cleaned (dry run)
    #[arg(long, help = "Only show what would be cleaned (dry run)")]
    pub dry_run: bool,

    /// Also clean backup archives (deprecated, use default behavior)
    #[arg(long, help = "Also remove archived sessions", hide = true)]
    pub backups: bool,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Show additional session details
    #[arg(long, short = 'v', help = "Show verbose session information")]
    pub verbose: bool,

    /// Show archived sessions
    #[arg(long, short = 'a', help = "Show archived sessions")]
    pub archived: bool,

    /// Quiet output (minimal formatting for completion)
    #[arg(long, short = 'q', help = "Quiet output for completion")]
    pub quiet: bool,
}

#[derive(Args, Debug)]
pub struct ResumeArgs {
    /// Session ID to resume (optional, shows list if not provided)
    pub session: Option<String>,

    /// Additional prompt or instructions for the resumed session
    #[arg(long, short)]
    pub prompt: Option<String>,

    /// Read additional instructions from specified file
    #[arg(long, short)]
    pub file: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct RecoverArgs {
    /// Session ID to recover from archive (optional, shows list if not provided)
    pub session: Option<String>,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: Option<ConfigCommands>,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Interactive configuration wizard
    Setup,
    /// Auto-detect and configure IDE
    Auto,
    /// Show current configuration
    Show,
    /// Edit configuration file
    Edit,
    /// Reset configuration to defaults
    Reset,
}

#[derive(Args, Debug)]
pub struct CompletionArgs {
    /// Shell to generate completion for, or 'init' for automatic setup
    pub shell: String,
}

#[derive(Args, Debug)]
pub struct MonitorArgs {}

#[derive(Args, Debug)]
pub struct StatusArgs {
    #[command(subcommand)]
    pub command: Option<StatusCommands>,

    /// Current task description (for backwards compatibility)
    pub task: Option<String>,

    /// Test status (passed, failed, unknown)
    #[arg(long, help = "Test status: passed, failed, or unknown")]
    pub tests: Option<String>,

    /// Confidence level (high, medium, low)
    #[arg(long, help = "Confidence level: high, medium, or low")]
    pub confidence: Option<String>,

    /// Todo progress (format: completed/total, e.g., 3/7)
    #[arg(long, help = "Todo progress in format 'completed/total' (e.g., '3/7')")]
    pub todos: Option<String>,

    /// Mark as blocked
    #[arg(long, help = "Mark session as blocked")]
    pub blocked: bool,

    /// Session name (optional, auto-detects from current directory)
    #[arg(long, help = "Session name (auto-detected if not provided)")]
    pub session: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum StatusCommands {
    /// Show status of one or all sessions
    Show {
        /// Session name (optional, shows all if not provided)
        session: Option<String>,

        /// Output format
        #[arg(long, help = "Output as JSON")]
        json: bool,
    },
    /// Generate a summary of all status files
    Summary {
        /// Output format
        #[arg(long, help = "Output as JSON")]
        json: bool,
    },
    /// Clean up stale status files
    Cleanup {
        /// Dry run - show what would be cleaned without removing
        #[arg(long, help = "Show what would be cleaned without removing")]
        dry_run: bool,
    },
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl StartArgs {
    pub fn validate(&self) -> crate::utils::Result<()> {
        if let Some(ref name) = self.name {
            validate_session_name(name)?;
        }
        Ok(())
    }
}

impl DispatchArgs {
    pub fn validate(&self) -> crate::utils::Result<()> {
        use std::io::IsTerminal;

        // Allow no arguments if stdin is piped
        if !std::io::stdin().is_terminal() {
            return Ok(());
        }

        self.validate_args()
    }

    fn validate_args(&self) -> crate::utils::Result<()> {
        match (&self.name_or_prompt, &self.prompt, &self.file) {
            (None, None, None) => Err(crate::utils::ParaError::invalid_args(
                "dispatch requires a prompt text or file path",
            )),
            _ => Ok(()),
        }
    }

    #[cfg(test)]
    pub fn validate_impl(&self, skip_stdin_check: bool) -> crate::utils::Result<()> {
        use std::io::IsTerminal;

        // Allow no arguments if stdin is piped (unless skipped for testing)
        if !skip_stdin_check && !std::io::stdin().is_terminal() {
            return Ok(());
        }

        self.validate_args()
    }
}

impl FinishArgs {
    pub fn validate(&self) -> crate::utils::Result<()> {
        if self.message.trim().is_empty() {
            return Err(crate::utils::ParaError::invalid_args(
                "Commit message cannot be empty",
            ));
        }

        if let Some(ref branch) = self.branch {
            validate_branch_name(branch)?;
        }

        Ok(())
    }
}

impl ResumeArgs {
    pub fn validate(&self) -> crate::utils::Result<()> {
        if self.prompt.is_some() && self.file.is_some() {
            return Err(crate::utils::ParaError::invalid_args(
                "Cannot specify both --prompt and --file. Please use only one.",
            ));
        }
        Ok(())
    }
}

pub fn validate_session_name(name: &str) -> crate::utils::Result<()> {
    if name.is_empty() {
        return Err(crate::utils::ParaError::invalid_args(
            "Session name cannot be empty",
        ));
    }

    if name.len() > 50 {
        return Err(crate::utils::ParaError::invalid_args(
            "Session name too long (max 50 characters)",
        ));
    }

    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(crate::utils::ParaError::invalid_args(
            "Session name can only contain alphanumeric characters, hyphens, and underscores",
        ));
    }

    Ok(())
}

pub fn validate_branch_name(name: &str) -> crate::utils::Result<()> {
    if name.is_empty() {
        return Err(crate::utils::ParaError::invalid_args(
            "Branch name cannot be empty",
        ));
    }

    if name.starts_with('-') || name.ends_with('-') {
        return Err(crate::utils::ParaError::invalid_args(
            "Branch name cannot start or end with hyphen",
        ));
    }

    if name.contains("..") || name.contains("//") {
        return Err(crate::utils::ParaError::invalid_args(
            "Branch name contains invalid character sequence",
        ));
    }

    Ok(())
}
